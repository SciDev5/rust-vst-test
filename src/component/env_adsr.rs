use crate::{util::{lx_interp::LXInterp, lerpable::Lerpable}, note::state::{NoteStateCurrentRaw, NoteState}};

pub struct ADSRSpec {
    attack: f32,
    attack_interp: LXInterp,
    
    decay: f32,
    decay_interp: LXInterp,
    
    sustain: f32,
    
    release: f32,
    release_interp: LXInterp,
}
impl ADSRSpec {
    pub fn linear(
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    ) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
            attack_interp: LXInterp::new(0.0),
            decay_interp: LXInterp::new(0.0),
            release_interp: LXInterp::new(0.0),
        }
    }
    fn value(&self, state: &NoteStateCurrentRaw) -> f32 {
        (
            if state.since_trigger < 0.0 {
                0.0
            } else if state.since_trigger < self.attack {
                self.attack_interp.interpolate_unity(state.since_trigger / self.attack)
            } else if state.since_trigger < self.attack + self.decay {
                self.decay_interp.interpolate_unity((state.since_trigger - self.attack) / self.decay)
                    .lerp(1.0, self.sustain)
            } else {
                self.sustain
            }
        ) * (
            if state.since_release < 0.0 {
                1.0
            } else if state.since_release < self.release {
                1.0 - self.release_interp.interpolate_unity(state.since_release / self.release)
            } else {
                0.0
            }
        )
    }
}

pub struct EnvelopeADSR {
    spec: ADSRSpec,
    pub buffer: Vec<f32>,
}
impl EnvelopeADSR {
    pub fn new(spec: ADSRSpec) -> Self {
        Self {
            spec,
            buffer: vec![],
        }
    }
    pub fn begin_block(&mut self) {
        self.buffer.clear();
    }
    pub fn update_block(&mut self, state: &NoteStateCurrentRaw) {
        self.buffer.push(self.spec.value(state));
    }
    
    pub fn update_note_ended(&self, state: &mut NoteState) {
        if state.seconds_since_released() > self.spec.release ||
            (self.spec.sustain == 0.0 && state.seconds_since_triggered() > self.spec.attack + self.spec.decay)
        {
            state.mark_ended();
        }
    }
}