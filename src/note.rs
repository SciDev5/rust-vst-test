use std::cmp::Ordering;

use nih_plug::prelude::SmoothingStyle;

use crate::{
    component::{
        env_adsr::{ADSRSpec, EnvelopeADSR},
        input_params::{InputFrequencyParam, InputParam},
        lfo::{LFOPhase, LFOSpec, LFO},
        noiseosc::{NoiseOscillator, NoiseOscillatorSpec, NoiseType, MultichunkWhiteNoiseGen},
        oscillator::{Oscillator, OscillatorSpec, UnisonSpec, UnisonFalloff, UnisonPhase},
        subosc::{SubOscillator, SubOscillatorSpec},
    },
    util::simple_waveforms::SimpleWaveform, common_data::CommonDataRef,
};

use self::{id::NoteId, state::NoteState};

pub mod id;
pub mod state;

pub struct Voice {
    state: NoteState,
    id: NoteId,

    pub freq: InputFrequencyParam,
    pub velocity: f32,
    pub aftertouch: InputParam,

    pub envs: [EnvelopeADSR; 2],
    pub lfos: [LFO; 4],
    pub subosc: SubOscillator,
    pub noiseosc: NoiseOscillator,
    pub osc_p: Oscillator,
    pub oscs: [Oscillator; 2],
}

impl Voice {
    pub fn new(
        sample_rate: f32,
        trigger_in: u32,
        id: NoteId,
        pitchbend: f32,
        aftertouch: f32,
        velocity: f32,

        data: CommonDataRef,
    ) -> Self {
        let mut self_ = Self {
            envs: [
                EnvelopeADSR::new(ADSRSpec::linear(0.005, 0.2, 0.5, 0.005)),
                EnvelopeADSR::new(ADSRSpec::linear(0.0, 0.0, 0.0, 0.0)),
            ],
            lfos: std::array::from_fn(|_| {
                LFO::new(
                    sample_rate,
                    LFOSpec::new(
                        0.0,
                        LFOPhase::AT(0.0),
                        SimpleWaveform::SINE,
                    ),
                )
            }),
            osc_p: Oscillator::new(sample_rate, OscillatorSpec::new(
                UnisonSpec::new(
                    4,
                    UnisonFalloff::Linear,
                    5.0,
                    UnisonPhase::Random,
                ),
                data.clone(),
            ), 440.0),
            oscs: [
                Oscillator::new(sample_rate, OscillatorSpec::new(
                    UnisonSpec::new(
                        4,
                        UnisonFalloff::Linear,
                        5.0,
                        UnisonPhase::Random,
                    ),
                    data.clone(),
                ), 440.0),
                Oscillator::new(sample_rate, OscillatorSpec::new(
                    UnisonSpec::new(
                        4,
                        UnisonFalloff::Linear,
                        5.0,
                        UnisonPhase::Random,
                    ),
                    data.clone(),
                ), 440.0),
            ],
            subosc: SubOscillator::new(sample_rate, SubOscillatorSpec::new(
                440.0,
                SimpleWaveform::SINE,
            )),
            noiseosc: NoiseOscillator::new(NoiseOscillatorSpec::new(
                NoiseType::MultichunkWhiteNoise(MultichunkWhiteNoiseGen::new()),
            )),
            
            freq: InputFrequencyParam::new(sample_rate, id.midi_note, pitchbend),
            velocity,
            aftertouch: InputParam::new(sample_rate, aftertouch, SmoothingStyle::Linear(2.0)),
            
            state: NoteState::new(sample_rate, trigger_in),
            id,
        };
        self_.reset();
        return self_;
    }

    fn reset(&mut self) {
        // reset smoothers here
    }
    pub fn kill(&mut self) {
        // release things
    }
    pub fn is_ended(&self) -> bool {
        self.state.ended
    }

    pub fn release(&mut self, in_samples: usize) {
        self.state.mark_released_in(in_samples as u32);
    }

    pub fn process(&mut self, out: &mut [Vec<f32>; 2]) {
        let block_len = out[0].len();
        let trigger_at = self.state.get_trigger_at();

        // :::::::::::::::::::::: ENVs :::::::::::::::::::::: //

        for env in &mut self.envs {
            env.begin_block();
        }
        for _ in 0 .. block_len {
            let current_state = self.state.current_raw();
            for env in &mut self.envs {
                env.update_block(&current_state);
            }
            self.state.tick();
        }
        self.envs[0].update_note_ended(&mut self.state);

        // :::::::::::::::::::::: LFOs :::::::::::::::::::::: //

        for lfo in &mut self.lfos {
            lfo.block(trigger_at, block_len);
        }

        // :::::::::::::::::::::: MOD OSCILLATOR :::::::::::::::::::::: //

        self.osc_p.block(trigger_at, block_len);

        // :::::::::::::::::::::: MAIN OSCILLATORs :::::::::::::::::::::: //

        for osc in &mut self.oscs {
            osc.block(trigger_at, block_len);
        }

        self.subosc.block(trigger_at, block_len);
        self.noiseosc.block(trigger_at, block_len);

        // :::::::::::::::::::::: EFFECTs :::::::::::::::::::::: //

        // todo

        // >>>>>>>>>> TEMP OUTPUT
        for i in 0 .. block_len {
            let gain = self.envs[0].buffer[i];
            out[0][i] = self.oscs[0].buffer[i] * gain;
            out[1][i] = self.oscs[1].buffer[i] * gain;
        }

    }

    pub fn sort_most_disposable_last(voices: &mut Vec<Voice>) {
        voices.sort_unstable_by(Self::ord_most_disposible);
    }
    fn ord_most_disposible(a: &Voice, b: &Voice) -> Ordering {
        if a.state.held != b.state.held {
            // If one is released and the other isn't,
            // the released one is more disposible.
            if !a.state.held {
                Ordering::Greater // 'a' released, more disposible
            } else {
                Ordering::Less
            }
        } else {
            // The one which started/ended longest ago as more disposable
            a.state
                .samples_since_changed()
                .cmp(&b.state.samples_since_changed())
        }
    }
    pub fn find_by_midi_note(voices: &mut Vec<Voice>, midi_note_id: u8) -> Option<&mut Voice> {
        for voice in voices {
            if voice.state.held && voice.id.midi_note == midi_note_id {
                return Some(voice);
            }
        }
        return None;
    }
    // pub fn find_by_voice_id(voices: &mut Vec<Voice>, voice_id: i32) -> Option<&mut Voice> {
    //     for voice in voices {
    //         if voice.id.voice_id == voice_id {
    //             return Some(voice);
    //         }
    //     }
    //     return None;
    // }
    pub fn find_all_by_channel(voices: &mut Vec<Voice>, channel: u8) -> Vec<&mut Voice> {
        let mut found: Vec<&mut Voice> = Vec::new();
        for voice in voices {
            if voice.id.channel == channel {
                found.push(voice);
            }
        }
        return found;
    }
}
