use std::{f32::consts, num::NonZeroU32, sync::Arc};

use nih_plug::{nih_export_vst3, prelude::*};

struct TestPlugin {
    params: Arc<TestParams>,
    sample_rate: f32,
    midi_note_id: u8,
    phase: f32,
    test_note_gain: Smoother<f32>,
}
impl TestPlugin {
    fn wave_from_phase(&self) -> f32 {
        (self.phase * consts::TAU).sin()
    }
    fn gen_wave(&mut self) -> f32 {
        self.phase += 100.0 / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0
        }

        self.wave_from_phase()
    }
}
impl Default for TestPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(TestParams::default()),
            phase: 0.0,
            midi_note_id: 0,
            sample_rate: 1.0,
            test_note_gain: Smoother::new(SmoothingStyle::Linear(5.0)),
        }
    }
}

#[derive(Params)]
struct TestParams {
    #[id = "gain"]
    pub gain: FloatParam,
}
impl Default for TestParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                -10.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 0.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0))
            .with_step_size(0.01)
            .with_unit(" dB"),
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
        self.midi_note_id = 0;

        true
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.test_note_gain.reset(0.0);
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
                if ev.timing() != sample_id as u32 {
                    break;
                }
                match ev {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.midi_note_id = note;

                        self.test_note_gain.set_target(self.sample_rate, velocity);
                    }
                    NoteEvent::NoteOff { note, .. } if note == self.midi_note_id => {
                        self.test_note_gain.set_target(self.sample_rate, 0.0);
                    }
                    NoteEvent::PolyPressure { .. } => {}
                    NoteEvent::PolyTuning { .. } => {}
                    _ => ()
                }
                midi_ev = context.next_event();
            }

            let gain_note = self.test_note_gain.next();
            let gain = util::db_to_gain_fast(self.params.gain.smoothed.next());

            let wave = self.gen_wave();

            let v = wave * gain * gain_note;

            for sample in samples {
                *sample = v;
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