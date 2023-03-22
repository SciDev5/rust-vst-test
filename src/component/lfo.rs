use crate::util::{simple_waveforms::SimpleWaveform, increment_mod::increment_phase, param_range::ParamRange};

use super::params::{ParamSource, ParamPolarity, Param};


pub enum LFOPhase {
    RANDOM,
    AT(f32),
}
impl LFOPhase {
    fn get_phase_value(&self) -> f32 {
        match self {
            Self::RANDOM => rand::random(),
            Self::AT(phase) => *phase,
        }
    }
}

pub struct LFOSpec {
    freq: f32,
    phase: LFOPhase,
    waveform: SimpleWaveform,
}
impl LFOSpec {
    pub fn new(
        freq: f32,
        phase: LFOPhase,
        waveform: SimpleWaveform,
    ) -> Self {
        Self { freq, phase, waveform }
    }
}

pub struct LFO {
    sample_rate: f32,

    buffer: Vec<f32>,
    spec: LFOSpec,
    phase: f32,
    pub freq: Param,
}

impl LFO {
    pub fn rangeof_freq() -> ParamRange { ParamRange::exponential(0.01, 100.0) }
    pub fn new(sample_rate: f32, spec: LFOSpec) -> Self {
        Self {
            sample_rate,
            buffer: vec![],
            freq: Param::new(spec.freq, Self::rangeof_freq()),
            phase: spec.phase.get_phase_value(),
            spec,
        }
    }
    pub fn update_spec(&mut self, spec: LFOSpec) {
        self.freq.rebase(spec.freq);
    }
    pub fn block(&mut self, trigger_at: usize, block_len: usize) {
        self.buffer.clear();
        let freq = self.freq.take(block_len);
        for i in 0 .. block_len {
            self.buffer.push(self.spec.waveform.sample(self.phase));
            if i >= trigger_at {
                increment_phase(&mut self.phase, self.sample_rate, freq[i]);
            }
        }
    }
}
impl ParamSource for LFO {
    const POLARITY: ParamPolarity = ParamPolarity::Bipolar;
    fn source_param_buffer(&self) -> &Vec<f32> {
        &self.buffer
    }
}