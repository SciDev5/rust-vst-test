use crate::util::{simple_waveforms::SimpleWaveform, increment_mod::increment_phase};


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

    pub buffer: Vec<f32>,
    spec: LFOSpec,
    phase: f32,
    freq: f32,
}

impl LFO {
    pub fn new(sample_rate: f32, spec: LFOSpec) -> Self {
        Self {
            sample_rate,
            buffer: vec![],
            freq: spec.freq,
            phase: spec.phase.get_phase_value(),
            spec,
        }
    }
    pub fn block(&mut self, trigger_at: usize, block_len: usize) {
        self.buffer.clear();
        for i in 0 .. block_len {
            self.buffer.push(self.spec.waveform.sample(self.phase));
            if i >= trigger_at {
                increment_phase(&mut self.phase, self.sample_rate, self.freq);
            }
        }
    }
}