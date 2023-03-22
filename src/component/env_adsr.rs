use crate::{util::{lx_interp::LXInterp, lerpable::Lerpable, param_range::ParamRange}, note::state::{NoteStateCurrentRaw, NoteState}};

use super::params::{ParamSource, ParamPolarity, ParamImmut};

pub struct ADSRSpec {
    attack: f32,
    attack_k: f32,
    
    decay: f32,
    decay_k: f32,
    
    sustain: f32,
    
    release: f32,
    release_k: f32,
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
            attack_k: 0.0,
            decay_k: 0.0,
            release_k: 0.0,
        }
    }
}

pub struct EnvelopeADSR {
    pub attack: ParamImmut,
    attack_interp: LXInterp,
    
    pub decay: ParamImmut,
    decay_interp: LXInterp,
    
    pub sustain: ParamImmut,
    
    pub release: ParamImmut,
    release_interp: LXInterp,
    
    buffer: Vec<f32>,
    adsrv: ADSRv,
}

impl EnvelopeADSR {
    pub fn rangeof_attack() -> ParamRange { ParamRange::exponential(100.0, 0.001) }
    pub fn rangeof_decay() -> ParamRange { ParamRange::exponential(100.0, 0.001) }
    pub fn rangeof_sustain() -> ParamRange { ParamRange::exponential_to_zero(1.0, 0.01) }
    pub fn rangeof_release() -> ParamRange { ParamRange::exponential(100.0, 0.001) }

    pub fn new(spec: ADSRSpec) -> Self {
        let attack = ParamImmut::new(spec.attack, Self::rangeof_attack());
        let decay = ParamImmut::new(spec.decay, Self::rangeof_decay());
        let sustain = ParamImmut::new(spec.sustain, Self::rangeof_sustain());
        let release = ParamImmut::new(spec.release, Self::rangeof_release());
        Self {
            adsrv: ADSRv { attack: attack.read(), decay: decay.read(), sustain: sustain.read(), release: release.read() },
            buffer: vec![],

            attack,
            attack_interp: LXInterp::new(spec.attack_k),
            decay,
            decay_interp: LXInterp::new(spec.decay_k),
            sustain,
            release,
            release_interp: LXInterp::new(spec.release_k),
        }
    }

    pub fn update_spec(&mut self, spec: ADSRSpec) {
        self.attack.rebase(spec.attack);
        self.attack_interp.set_k(spec.attack_k);
        self.decay.rebase(spec.decay);
        self.decay_interp.set_k(spec.decay_k);
        self.sustain.rebase(spec.sustain);
        self.release.rebase(spec.release);
        self.release_interp.set_k(spec.release_k);

        self.adsrv = ADSRv { attack: self.attack.read(), decay: self.decay.read(), sustain: self.sustain.read(), release: self.release.read() };
    }

    pub fn begin_block(&mut self) {
        self.buffer.clear();
    }
    pub fn update_block(&mut self, state: &NoteStateCurrentRaw) {
        self.buffer.push(self.value(state));
    }
    
    pub fn update_note_ended(&self, state: &mut NoteState) {
        let ADSRv { attack, decay, sustain, release } = self.adsrv;
        if state.seconds_since_released() > release ||
            (sustain == 0.0 && state.seconds_since_triggered() > attack + decay)
        {
            state.mark_ended();
        }
    }

    fn value(&self, state: &NoteStateCurrentRaw) -> f32 {
        let ADSRv { attack, decay, sustain, release } = self.adsrv;
        (
            if state.since_trigger < 0.0 {
                0.0
            } else if state.since_trigger < attack {
                self.attack_interp.interpolate_unity(state.since_trigger / attack)
            } else if state.since_trigger < attack + decay {
                self.decay_interp.interpolate_unity((state.since_trigger - attack) / decay)
                    .lerp(1.0, sustain)
            } else {
                sustain
            }
        ) * (
            if state.since_release < 0.0 {
                1.0
            } else if state.since_release < release {
                1.0 - self.release_interp.interpolate_unity(state.since_release / release)
            } else {
                0.0
            }
        )
    }
}
impl ParamSource for EnvelopeADSR {
    const POLARITY: ParamPolarity = ParamPolarity::Monopolar;
    fn source_param_buffer(&self) -> &Vec<f32> {
        &self.buffer
    }
}

struct ADSRv {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}