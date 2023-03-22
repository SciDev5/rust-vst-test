use crate::util::{simple_waveforms::SimpleWaveform, increment_mod::increment_phase, param_range::ParamRange};

use super::params::{ParamSource, ParamPolarity, Param};


pub struct SubOscillatorSpec {
    freq_off: f32,
    waveform: SimpleWaveform,
}
impl SubOscillatorSpec {
    pub fn new(
        freq_off: f32,
        waveform: SimpleWaveform,
    ) -> Self {
        Self { freq_off, waveform }
    }
}

pub struct SubOscillator {
    sample_rate: f32,

    buffer: Vec<f32>,
    spec: SubOscillatorSpec,
    phase: f32,

    pub freq: Param,
}

impl SubOscillator {
    pub fn rangeof_freq() -> ParamRange { ParamRange::exponential(0.5, 20000.0) }
    pub fn new(sample_rate: f32, spec: SubOscillatorSpec) -> Self {
        Self {
            sample_rate,
            buffer: vec![],
            freq: Param::new(spec.freq_off, Self::rangeof_freq()),
            spec,
            phase: 0.0,
        }
    }
    pub fn update_spec(&mut self, spec: SubOscillatorSpec) {
        self.freq.rebase(spec.freq_off);
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
impl ParamSource for SubOscillator {
    const POLARITY: ParamPolarity = ParamPolarity::Bipolar;
    fn source_param_buffer(&self) -> &Vec<f32> {
        &self.buffer
    }
}