use std::sync::Arc;
use std::sync::atomic::AtomicI64;

use nih_plug::prelude::{Params, FloatParam, FloatRange, SmoothingStyle};
use nih_plug_vizia::ViziaState;

use crate::editor;
use crate::state::TextState;

#[derive(Params)]
pub struct TestParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[persist = "yeet-lol"]
    pub rel: Arc<TextState>,

    #[persist = "yeet-lol-id"]
    pub rel_id: Arc<AtomicI64>,

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

            editor_state: editor::default_state(),

            rel: Arc::new(TextState::default()),
            rel_id: Arc::new(AtomicI64::new(0)),
        }
    }
}
