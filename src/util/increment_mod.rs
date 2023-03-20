pub fn increment_mod_01_f32(var: &mut f32, delta: f32) {
    *var += delta;
    if *var >= 1.0 {
        *var -= 1.0;
    }
    if *var < 0.0 {
        *var += 1.0;
    }
}

pub fn increment_phase(phase: &mut f32, sample_rate: f32, freq: f32) {
    increment_mod_01_f32(phase, freq/sample_rate);
}