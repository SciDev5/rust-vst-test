use crate::util::{simple_waveforms::SimpleWaveform, increment_mod::increment_phase};


pub struct SubOscillatorSpec {
    freq: f32,
    waveform: SimpleWaveform,
}
impl SubOscillatorSpec {
    pub fn new(
        freq: f32,
        waveform: SimpleWaveform,
    ) -> Self {
        Self { freq, waveform }
    }
}

pub struct SubOscillator {
    sample_rate: f32,

    pub buffer: Vec<f32>,
    spec: SubOscillatorSpec,
    phase: f32,
    freq: f32,
}

impl SubOscillator {
    pub fn new(sample_rate: f32, spec: SubOscillatorSpec) -> Self {
        Self {
            sample_rate,
            buffer: vec![],
            freq: spec.freq,
            spec,
            phase: 0.0,
        }
    }
    pub fn block(&mut self, trigger_at: usize, block_len: usize) {
        self.buffer.clear();
        for i in 0 .. block_len {
            self.buffer.push(self.spec.waveform.sample(self.phase));
            if i >= trigger_at {
                increment_phase(&mut self.phase, self.sample_rate, self.freq)
            }
        }
    }
}