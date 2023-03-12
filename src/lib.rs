use std::{num::NonZeroU32, sync::Arc};

use nih_plug::{nih_export_vst3, prelude::*};

mod params;
mod note;
use params::TestParams;
use note::*;

struct TestPlugin {
    params: Arc<TestParams>,
    sample_rate: f32,

    voices: [note::NotePlayer; 5],
}
impl TestPlugin {
    #[inline(always)]
    fn for_each_voice<T>(&mut self, mut cb: T)
    where
        T: FnMut(&mut NotePlayer) -> (),
    {
        for voice in self.voices.iter_mut() {
            cb(voice)
        }
    }
    fn wave(&mut self) -> [f32; 2] {
        let mut sum: [f32; 2] = [0.0, 0.0];

        self.for_each_voice(|voice| {
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
        Self {
            params: Arc::new(TestParams::default()),
            sample_rate: 1.0,
            voices: std::array::from_fn(|_| NotePlayer::default()),
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

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.for_each_voice(|it| it.init(buffer_config));

        true
    }

    fn reset(&mut self) {
        self.for_each_voice(|it| it.reset());
    }

    fn params(&self) -> std::sync::Arc<dyn Params> {
        self.params.clone()
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
                        voice_id,
                        ..
                    } => {
                        if let Some(current_note) = NotePlayer::find_by_held_note(
                            &mut self.voices,
                            note,
                        ) {
                            current_note.release()
                        }
                        NotePlayer::find_to_trigger(&mut self.voices).trigger(
                            voice_id.unwrap_or(-1),
                            note,
                            velocity,
                        );
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        if let Some(current_note) = NotePlayer::find_by_held_note(
                            &mut self.voices,
                            note,
                        ) {
                            current_note.release()
                        }
                    }
                    NoteEvent::PolyPressure { .. } => {}
                    NoteEvent::PolyTuning { .. } => {}
                    _ => (),
                }
                midi_ev = context.next_event();
            }

            let wave = self.wave();
            let gain = util::db_to_gain_fast(self.params.gain.smoothed.next());

            for (i, sample) in samples.into_iter().enumerate() {
                *sample = wave[i] * gain;
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
