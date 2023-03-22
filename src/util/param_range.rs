use super::lerpable::Lerpable;


pub enum ParamRange {
    Linear { min: f32, max: f32 },
    Exponential { min: f32, max: f32, base: f32 },
    ExponentialToZero { virtual_min: f32, max: f32, base: f32 },
}
impl ParamRange {
    pub fn exponential(min: f32, max: f32) -> ParamRange {
        Self::Exponential { min, max, base: (max/min).ln() }
    }
    pub fn exponential_to_zero(max: f32, virtual_min: f32) -> ParamRange {
        Self::ExponentialToZero { virtual_min, max, base: ((max+virtual_min)/virtual_min).ln() }
    }
    pub fn linear(min: f32, max: f32) -> ParamRange {
        Self::Linear { min, max }
    }
    pub fn normalize(&self, x: f32) -> f32 {
        match self {
            Self::Linear { min, max } =>
                x.invlerp(*min, *max).clamp(0.0, 1.0),
            Self::Exponential { min, base, .. } => 
                (x/min).ln()/base,
            Self::ExponentialToZero { virtual_min, base, .. } => 
                ((x+virtual_min)/virtual_min).ln()/base,
        }.clamp(0.0, 1.0)
    }
    pub fn denormalize(&self, y: f32) -> f32 {
        let y = y.clamp(0.0, 1.0);
        match self {
            Self::Linear { min, max } =>
                y.lerp(*min, *max),
            Self::Exponential { min, base, .. } => 
                min * f32::exp(base * y),
            Self::ExponentialToZero { virtual_min, base, .. }  => 
                virtual_min * f32::exp(base * y) - virtual_min,
        }
    }
}
