

pub trait Lerpable<Bound> {
    fn lerp(&self, lower: Bound, upper: Bound) -> Self;
}
impl Lerpable<f32> for f32 {
    fn lerp(&self, lower: f32, upper: f32) -> Self {
        return lower * (1.0 - self) + upper * self;
    }
}