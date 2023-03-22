use crate::{util::{increment_mod::increment_phase, param_range::ParamRange}, common_data::CommonDataRef};

use super::params::{ParamSource, ParamPolarity, Param};

pub enum UnisonFalloff {
    Linear,
}
impl UnisonFalloff {
    fn value(&self, distance: f32) -> f32 {
        match self {
            Self::Linear => distance,
        }
    }
}
pub enum UnisonPhase {
    Random,
    Zero,
}
impl UnisonPhase {
    fn value(&self) -> f32 {
        match self {
            Self::Random => rand::random(),
            Self::Zero => 0.0,
        }
    }
}
pub struct UnisonSpec {
    n_voices: u8,
    falloff: UnisonFalloff,
    /// maximum detune in cents
    detune: f32,
    phase: UnisonPhase,
}
impl UnisonSpec {
    pub fn new(
        n_voices: u8,
        falloff: UnisonFalloff,
        detune: f32,
        phase: UnisonPhase,
    ) -> Self {
        Self { n_voices, falloff, detune, phase }
    }
    fn into_voices(&self) -> Vec<UnisonVoice> {
        let n = self.n_voices as usize;
        let mut voices = Vec::with_capacity(n);
        
        for i in 0 .. n {
            // ranges `(0, 1]`, and is `1` at the center.
            let a = (1 + usize::min(i,n-i-1)) as f32 / ((n+1)/2) as f32;
            // ranges `[-1,1]`, and is `0` at the center.
            let b = (2 * i) as f32 / (n - 1) as f32 - 1.0;
            voices.push(
                UnisonVoice {
                    phase: self.phase.value(),
                    gain: self.falloff.value(a),
                    freq_off: (self.detune / 1200.0 * b).exp2(),
                }
            );
        }

        voices
    } 
}
struct UnisonVoice {
    phase: f32,
    gain: f32,
    freq_off: f32,
}
impl UnisonVoice {
    fn step(&mut self, sample_rate: f32, base_freq: f32) {
        increment_phase(&mut self.phase, sample_rate, base_freq * self.freq_off);
    }
}


pub struct OscillatorSpec {
    unison_spec: UnisonSpec,
    data: CommonDataRef,
    freq_off: f32,
    slice: f32,
}
impl OscillatorSpec {
    pub fn new(
        unison_spec: UnisonSpec,
        data: CommonDataRef,
        freq_off: f32,
        slice: f32,
    ) -> Self {
        Self { unison_spec, data, freq_off, slice }
    }
}

pub struct Oscillator {
    sample_rate: f32,

    buffer: Vec<f32>,
    spec: OscillatorSpec,

    voices: Vec<UnisonVoice>,

    pub slice: Param,
    pub freq: Param,
}

impl Oscillator {
    pub fn rangeof_freq() -> ParamRange { ParamRange::exponential(0.5, 20000.0) }
    pub fn rangeof_slice() -> ParamRange { ParamRange::linear(0.0, 1.0) }
    pub fn new(sample_rate: f32, spec: OscillatorSpec) -> Self {
        Self {
            sample_rate,

            buffer: vec![],
            
            voices: spec.unison_spec.into_voices(),
            
            slice: Param::new(spec.slice, Self::rangeof_slice()),
            freq: Param::new(spec.freq_off, Self::rangeof_freq()),

            spec,
        }
    }
    pub fn update_spec(&mut self, spec: OscillatorSpec) {
        self.freq.rebase(spec.freq_off);
        self.slice.rebase(spec.slice);
        // TODO update voices.
    }
    pub fn block(&mut self, trigger_at: usize, block_len: usize) {
        self.buffer.clear();
        
        let wavetable = &self.spec.data.lock().unwrap().wavetable;
        let slice = self.slice.take(block_len);
        let freq = self.freq.take(block_len);
        for i in 0 .. block_len {
            let mut value = 0.0;
            for voice in &self.voices {
                value += wavetable.data.sample(voice.phase, slice[i]) * voice.gain;
            }
            self.buffer.push(value);

            if i > trigger_at {
                for voice in &mut self.voices {
                    voice.step(self.sample_rate, freq[i]);
                }
            }
        }
    }
}
impl ParamSource for Oscillator {
    const POLARITY: ParamPolarity = ParamPolarity::Bipolar;
    fn source_param_buffer(&self) -> &Vec<f32> {
        &self.buffer
    }
}