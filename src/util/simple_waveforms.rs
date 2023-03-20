pub enum SimpleWaveform {
    SINE,
    SAW,
    SQUARE,
    TRIANGLE,
}
impl SimpleWaveform {
    pub fn sample(&self, phase: f32) -> f32 {
        match self {
            Self::SINE => (phase * std::f32::consts::TAU).sin(),
            Self::SAW => phase * 2.0 - 1.0,
            Self::SQUARE => if phase > 0.5 { 1.0 } else { -1.0 },
            Self::TRIANGLE => f32::abs(((4.0*phase + 3.0) % 4.0) - 2.0) - 1.0
        }
    }
}