use std::{
    num::NonZeroU32,
    path::Path,
    sync::{atomic::Ordering, Arc, Mutex},
};

use atomic_float::AtomicF32;
use common_data::{CommonDataRef, CommonData};
use nih_plug::{nih_export_vst3, prelude::*};

mod component;
mod dbug;
mod editor;
mod note;
mod params;
mod state;
mod util;
mod common_data;

use component::wavetable::{Wav, Wavetable};
use note::{id::NoteId, *};
use params::TestParams;

const MAX_POLYPHONY: usize = 16;

const MIDI_SPEC_CHANNEL_COUNT: usize = 16;

struct TestPlugin {
    params: Arc<TestParams>,
    sample_rate: f32,

    voices: Vec<Voice>,
    channel_tunings: [f32; MIDI_SPEC_CHANNEL_COUNT],
    channel_aftertouch: [f32; MIDI_SPEC_CHANNEL_COUNT],

    peak_meter: Arc<AtomicF32>,

    data: CommonDataRef,
    last_rel_id: i64,
}
impl TestPlugin {
    fn update_wave(&mut self) {
        let rel_id = self.params.rel_id.load(Ordering::Relaxed);
        if rel_id == self.last_rel_id {
            return;
        } else {
            self.last_rel_id = rel_id;
        }
        let path = self.params.rel.get_v();

        if let Some(wav) = Wav::from_filepath(&Path::new(&path)) {
            if let Some(wav) = Wavetable::slice_downsample(&wav, 2048) {
                self.data.lock().unwrap().wavetable = wav;
            }
        }
    }

    fn kill_voice(&mut self, i: usize) {
        self.voices.remove(i).kill();
    }
}
impl Default for TestPlugin {
    fn default() -> Self {
        let data: CommonDataRef = Arc::new(Mutex::new(CommonData {
            wavetable: Wavetable::default(),
        }));

        Self {
            params: Arc::new(TestParams::default()),
            sample_rate: 1.0,

            voices: vec![],
            channel_tunings: [0.0; 16],
            channel_aftertouch: [0.0; 16],

            peak_meter: Arc::new(AtomicF32::new(nih_plug::prelude::util::MINUS_INFINITY_DB)),

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

        true
    }

    fn reset(&mut self) {
        // clear internal state here
        {
            for i in (0..self.voices.len()).rev() {
                self.kill_voice(i);
            }
            self.voices.clear();
        }
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
        let block_length = buffer.samples();

        // :::::::::::::::::::::: MIDI PROCESSING :::::::::::::::::::::: //

        for voice in &mut self.voices {
            voice.freq.pitchbend.begin_block();
            voice.aftertouch.begin_block();
        }
        for sample_id in 0 .. block_length {
            while let Some(ev) = midi_ev {
                if ev.timing() > sample_id as u32 {
                    break;
                }
                match ev {
                    NoteEvent::Choke { note, .. } => {
                        if let Some(current_note) = Voice::find_by_midi_note(&mut self.voices, note)
                        {
                            current_note.choke(sample_id);
                        }
                    }
                    NoteEvent::NoteOn {
                        note,
                        velocity,
                        channel,
                        voice_id,
                        ..
                    } => {
                        if let Some(current_note) = Voice::find_by_midi_note(&mut self.voices, note)
                        {
                            current_note.release(sample_id);
                        }

                        // Make space if needed.
                        Voice::sort_most_disposable_last(&mut self.voices);
                        while self.voices.len() > MAX_POLYPHONY - 1 {
                            self.kill_voice(MAX_POLYPHONY - 1);
                        }

                        self.voices.push(Voice::new(
                            self.sample_rate,
                            sample_id as u32,
                            NoteId {
                                midi_note: note,
                                voice_id: voice_id.unwrap_or_default(),
                                channel,
                            },
                            self.channel_tunings[channel as usize],
                            self.channel_aftertouch[channel as usize],
                            velocity,

                            self.data.clone(),
                        ));
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        if let Some(current_note) = Voice::find_by_midi_note(&mut self.voices, note)
                        {
                            current_note.release(sample_id);
                        }
                    }
                    NoteEvent::MidiChannelPressure {
                        pressure, channel, ..
                    } => {
                        self.channel_aftertouch[channel as usize] = pressure;
                        for note in Voice::find_all_by_channel(&mut self.voices, channel) {
                            note.aftertouch.update_block(sample_id, pressure);
                        }
                    }
                    NoteEvent::MidiPitchBend { channel, value, .. } => {
                        let tuning = (value * 256.0 - 128.0) / 8.0 * 3.0;
                        self.channel_tunings[channel as usize] = tuning;
                        for note in Voice::find_all_by_channel(&mut self.voices, channel) {
                            note.freq.pitchbend.update_block(sample_id, tuning);
                        }
                    }
                    _ => (),
                };
                midi_ev = context.next_event();
            }
        }
        for voice in &mut self.voices {
            voice.freq.pitchbend.finalize_block(block_length);
            voice.aftertouch.finalize_block(block_length);
        }

        // :::::::::::::::::::::: PROCESS VOICES :::::::::::::::::::::: //
        let mut out = [vec![0.0; block_length], vec![0.0; block_length]];

        for voice in &mut self.voices {
            voice.process(&mut out);
        }

        for (sample_id, samples) in buffer.iter_samples().enumerate() {
            for (i, sample) in samples.into_iter().enumerate() {
                *sample = out[i][sample_id];
            }
        }

        let mut indices_to_drop = vec![];
        for (i,_) in (&self.voices)
            .into_iter().enumerate()
            .filter(|(_,voice)| voice.is_ended()) {
                indices_to_drop.push(i);
        }
        indices_to_drop.sort();
        indices_to_drop.reverse();
        for i in indices_to_drop {
            self.kill_voice(i);
        }

        // :::::::::::::::::::::: UI UPDATES :::::::::::::::::::::: //

        // To save resources, a plugin can (and probably should!) only perform expensive
        // calculations that are only displayed on the GUI while the GUI is open
        if self.params.editor_state.is_open() {
            self.update_wave();
            for sample_id in 0 .. block_length {
                let wave:[f32; 2] = std::array::from_fn(|i| out[i][sample_id]);

                let amplitude = (wave[0].abs() + wave[1].abs()) / 2.0;
                let current_peak_meter = self.peak_meter.load(std::sync::atomic::Ordering::Relaxed);
                let new_peak_meter = if amplitude > current_peak_meter {
                    amplitude
                } else {
                    const PEAK_METER_DECAY_WEIGHT: f32 = 0.99;
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
