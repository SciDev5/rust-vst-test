use nih_plug::prelude::*;

#[derive(Params)]
pub struct TestParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for TestParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                -10.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 0.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0))
            .with_step_size(0.01)
            .with_unit(" dB"),
        }
    }
}