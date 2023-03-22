use super::params::{ParamSource, ParamPolarity};


pub struct MultichunkWhiteNoiseGen {
    buffers: Vec<[f32; Self::L]>,
    sample_i: usize,
    chunk_i: usize,
}
impl MultichunkWhiteNoiseGen {
    const L: usize = 512;
    const SL: usize = 128;
    pub fn new() -> Self {        
        let mut buffers = vec![];
        for _ in 0 .. Self::SL {
            buffers.push(std::array::from_fn(|_| rand::random::<f32>() * 2.0 - 1.0));
        }
        Self {
            buffers,
            sample_i: 0,
            chunk_i: Self::gen_chunk_i(),
        }
    }
    fn gen_chunk_i() -> usize {
        (rand::random::<f32>() * Self::SL as f32) as usize
    }
    pub fn sample(&mut self) -> f32 {
        self.sample_i += 1;
        if self.sample_i >= Self::L {
            self.sample_i = 0;
            self.chunk_i = Self::gen_chunk_i();
        }
        self.buffers[self.chunk_i][self.sample_i]
    }
}

pub enum NoiseType {
    MultichunkWhiteNoise(MultichunkWhiteNoiseGen),
}

pub struct NoiseOscillatorSpec {
    noise_type: NoiseType,
}
impl NoiseOscillatorSpec {
    pub fn new(
        noise_type: NoiseType,
    ) -> Self {
        Self { noise_type }
    }
}

pub struct NoiseOscillator {
    spec: NoiseOscillatorSpec,
    buffer: Vec<f32>,
}
impl NoiseOscillator {
    pub fn new(spec: NoiseOscillatorSpec) -> Self {
        Self {
            spec,
            buffer: vec![],
        }
    }
    pub fn block(&mut self, _trigger_at: usize, block_len: usize) {
        self.buffer.clear();
        for _ in 0 .. block_len {
            self.buffer.push(match &mut self.spec.noise_type {
                NoiseType::MultichunkWhiteNoise(gen) => gen.sample(),
            });
        }
    }
}
impl ParamSource for NoiseOscillator {
    const POLARITY: ParamPolarity = ParamPolarity::Bipolar;
    fn source_param_buffer(&self) -> &Vec<f32> {
        &self.buffer
    }
}