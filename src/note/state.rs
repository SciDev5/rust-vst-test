
pub struct NoteState {
    sample_rate: f32,
    pub held: bool,
    trigger_in: u32,
    release_in: u32,
    pub samples_since_trigger: u32,
    pub samples_since_release: u32,

    pub ended: bool,
}

impl NoteState {
    pub fn new(sample_rate: f32, trigger_in: u32) -> Self {
        Self {
            sample_rate,
            held: true,
            samples_since_trigger: 0,
            samples_since_release: 0,
            trigger_in,
            release_in: 0,
            ended: false,
        }
    }
    pub fn get_trigger_at(&self) -> usize {
        self.trigger_in as usize
    }
    pub fn mark_released_in(&mut self, release_in: u32) {
        self.release_in = release_in;
    }
    pub fn mark_ended(&mut self) {
        self.ended = true;
    }
    pub fn tick(&mut self) {
        if self.release_in > 0 {
            self.release_in -= 1;
            if self.release_in == 0 { // this is inside so we don't release instantly.
                self.held = false;
            }
        }
        if self.trigger_in > 0 {
            self.trigger_in -= 1;
            return;
        }

        self.samples_since_trigger += 1;
        if !self.held {
            self.samples_since_release += 1;
        }
    }
    pub fn has_triggered(&self) -> bool {
        self.trigger_in == 0
    }
    pub fn samples_since_changed(&self) -> u32 {
        if self.held {
            self.samples_since_trigger
        } else {
            self.samples_since_release
        }
    }
    pub fn seconds_since_triggered(&self) -> f32 {
        self.samples_since_trigger as f32 / self.sample_rate
    }
    pub fn seconds_since_released(&self) -> f32 {
        self.samples_since_release as f32 / self.sample_rate
    }
    pub fn current_raw(&self) -> NoteStateCurrentRaw {
        NoteStateCurrentRaw {
            has_triggered: self.has_triggered(),
            since_trigger: self.seconds_since_triggered(),
            since_release: self.seconds_since_released(),
        }
    }
}

pub struct NoteStateCurrentRaw {
    pub has_triggered: bool,
    pub since_trigger: f32,
    pub since_release: f32,
}