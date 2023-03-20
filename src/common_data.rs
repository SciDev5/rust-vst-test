use std::sync::{Arc, Mutex};

use crate::component::wavetable::Wavetable;

pub type CommonDataRef = Arc<Mutex<CommonData>>;

pub struct CommonData {
    pub wavetable: Wavetable,
}