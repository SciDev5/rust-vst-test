pub struct LXInterp {
    positive: bool,
    k: f32,
}
impl LXInterp {
    pub fn new(fac: f32) -> Self {
        Self {
            positive: fac >= 0.0,
            k: fac.abs(),
        }
    }
    pub fn interpolate_unity(&self, x: f32) -> f32 {
        x + if self.positive {
            (1.0 - x) * (1.0 - f32::exp(-self.k * x))
        } else {
            (-x) * (1.0 - f32::exp(self.k * (x - 1.0)))
        }
    }
    pub fn set_k(&mut self, k: f32) {
        self.positive = k >= 0.0;
        self.k = k.abs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_close(a: f32, b: f32, distance: f32) -> bool {
        (a-b).abs() <= distance
    }

    #[test]
    fn linearity_when_k_is_0() {
        let lxi = LXInterp::new(0.0);
        for i in 0 ..= 4 {
            let x = i as f32 / 4.0;
            
            assert!(is_close(x, lxi.interpolate_unity(x),0.001));
        }
    }
}