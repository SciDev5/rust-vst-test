use std::{num::NonZeroU32, sync::{Arc, Mutex, atomic::Ordering}, path::Path};

use atomic_float::AtomicF32;
use common_data::{CommonDataRef, CommonData};
use nih_plug::{nih_export_vst3, prelude::*};

mod params;
mod note;
mod editor;
mod state;
mod wavetable;
mod common_data;

use params::TestParams;
use note::*;
use wavetable::Wavetable;


const N_NOTEPLAYERS:usize = 5;

struct TestPlugin {
    params: Arc<TestParams>,
    sample_rate: f32,

    noteplayers: [note::NotePlayer; N_NOTEPLAYERS],
    channel_tunings: [f32; 16],
    channel_aftertouch: [f32; 16],

    peak_meter: Arc<AtomicF32>,

    data: CommonDataRef,
    last_rel_id: i64,
}
impl TestPlugin {
    #[inline(always)]
    fn for_each_noteplayer<T>(&mut self, mut cb: T)
    where
        T: FnMut(&mut NotePlayer) -> (),
    {
        for voice in self.noteplayers.iter_mut() {
            cb(voice)
        }
    }
    fn update_wave(&mut self) {
        let path = self.params.rel.get_v();
        let rel_id = self.params.rel_id.load(Ordering::Relaxed);
        if rel_id == self.last_rel_id {
            return;
        } else {
            self.last_rel_id = rel_id;
        }

        if let Some(wav) = wavetable::Wav::from_filepath(&Path::new(&path)) {
            if let Some(wav) = Wavetable::new(wav, 2048) {
                self.data.lock().unwrap().wavetable = wav;
            }
        }

    }
    fn wave(&mut self) -> [f32; 2] {
        let mut sum: [f32; 2] = [0.0, 0.0];

        self.for_each_noteplayer(|voice| {
            let voice_wave = voice.next();
            for i in 0..2 {
                sum[i] += voice_wave[i];
            }
        });

        sum
    }
}
impl Default for TestPlugin {
    fn default() -> Self {
        let data: CommonDataRef = Arc::new(Mutex::new(CommonData {
            wavetable: Wavetable::default(),
        }));
        let mut noteplayers = std::array::from_fn(|_| None);
        for i in 0 .. N_NOTEPLAYERS {
            noteplayers[i] = Some(NotePlayer::new(
                data.clone()
            ))
        }
        let noteplayers = noteplayers.map(|it| it.unwrap());

        Self {
            params: Arc::new(TestParams::default()),
            sample_rate: 1.0,
            noteplayers,
            channel_tunings: [0.0; 16],
            channel_aftertouch: [0.0; 16],

            peak_meter: Arc::new(AtomicF32::new(util::MINUS_INFINITY_DB)),

            data,
            last_rel_id: 0,
        }
    }
}

impl Plugin for TestPlugin {
    const NAME: &'static str = "TestPlugin";

    const VENDOR: &'static str = "SciDev5";

    const URL: &'static str = "no";

    const EMAIL: &'static str = "no";

    const VERSION: &'static str = "0.0.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.update_wave();
        self.for_each_noteplayer(|it| it.init(buffer_config));

        true
    }

    fn reset(&mut self) {
        self.for_each_noteplayer(|it| it.reset());
    }

    fn params(&self) -> std::sync::Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.params.rel.clone(),
            self.params.rel_id.clone(),
            self.peak_meter.clone(),
            self.params.editor_state.clone(),
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut midi_ev = context.next_event();

        for (sample_id, samples) in buffer.iter_samples().enumerate() {
            while let Some(ev) = midi_ev {
                if ev.timing() > sample_id as u32 {
                    break;
                }
                match ev {
                    NoteEvent::NoteOn {
                        note,
                        velocity,
                        channel,
                        voice_id,
                        ..
                    } => {
                        if let Some(current_note) = NotePlayer::find_by_held_note(
                            &mut self.noteplayers,
                            note,
                        ) {
                            current_note.release();
                        }
                        let noteplayer = NotePlayer::find_to_trigger(&mut self.noteplayers);
                        noteplayer.trigger(
                            channel,
                            voice_id.unwrap_or_default(),
                            note,
                            velocity,
                        );
                        noteplayer.tuning(self.channel_tunings[channel as usize]);
                        noteplayer.pressure(self.channel_aftertouch[channel as usize])
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        if let Some(current_note) = NotePlayer::find_by_held_note(
                            &mut self.noteplayers,
                            note,
                        ) {
                            current_note.release()
                        }
                    }
                    NoteEvent::MidiChannelPressure { pressure, channel, .. } => {
                        self.channel_aftertouch[channel as usize] = pressure;
                        for note in NotePlayer::find_all_by_channel(&mut self.noteplayers, channel) {
                            note.pressure(pressure);
                        }
                    }
                    NoteEvent::MidiPitchBend { channel, value, .. } => {
                        let tuning = (value*256.0-128.0)/8.0*3.0;
                        self.channel_tunings[channel as usize] = tuning;
                        for note in NotePlayer::find_all_by_channel(&mut self.noteplayers, channel) {
                            note.tuning(tuning);
                        }
                    }
                    _ => (),
                }
                midi_ev = context.next_event();
            }


            let wave = self.wave();
            let gain = util::db_to_gain_fast(self.params.gain.smoothed.next());

            for (i, sample) in samples.into_iter().enumerate() {
                *sample = wave[i] * gain;
            }

            // To save resources, a plugin can (and probably should!) only perform expensive
            // calculations that are only displayed on the GUI while the GUI is open
            if self.params.editor_state.is_open() {
                self.update_wave();

                let amplitude = wave[0].abs();
                let current_peak_meter = self.peak_meter.load(std::sync::atomic::Ordering::Relaxed);
                let new_peak_meter = if amplitude > current_peak_meter {
                    amplitude
                } else {
                    const PEAK_METER_DECAY_WEIGHT:f32 = 0.99;
                    current_peak_meter * PEAK_METER_DECAY_WEIGHT
                        + amplitude * (1.0 - PEAK_METER_DECAY_WEIGHT)
                };

                self.peak_meter
                    .store(new_peak_meter, std::sync::atomic::Ordering::Relaxed)
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for TestPlugin {
    const CLAP_ID: &'static str = "me.scidev5";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("funky lmao");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}
impl Vst3Plugin for TestPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"TestPlugin______";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument];
}

nih_export_clap!(TestPlugin);
nih_export_vst3!(TestPlugin);
