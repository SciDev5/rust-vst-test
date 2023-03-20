use nih_plug::prelude::{Smoother, SmoothingStyle};

pub struct InputFrequencyParam {
    midi_note: u8,
    pub pitchbend: InputParam,
    // modulation_oct: f32,
}

impl InputFrequencyParam {
    pub fn new(
        sample_rate: f32,
        midi_note: u8,
        start_pitchbend: f32,
    ) -> Self {
        Self {
            midi_note,
            pitchbend: InputParam::new(sample_rate, start_pitchbend, SmoothingStyle::None),
        }
    }
}


pub struct InputParam {
    sample_rate: f32,
    current: Smoother<f32>,
    buffer: Vec<f32>,
}

impl InputParam {
    pub fn new(
        sample_rate: f32,
        start_value: f32,
        smoothing_style: SmoothingStyle,
    ) -> Self {
        let current = Smoother::new(smoothing_style);
        current.reset(start_value);
        Self {
            sample_rate,
            current,
            buffer: vec![],
        }
    }
    fn extend_buffer_to_len(&mut self, sample_id: usize) {
        while self.buffer.len() < sample_id {
            self.buffer.push(self.current.next());
        }
    }
    pub fn begin_block(&mut self) {
        self.buffer.clear();
    }
    pub fn update_block(&mut self, sample_id: usize, new_value: f32) {
        self.extend_buffer_to_len(sample_id);
        self.current.set_target(self.sample_rate, new_value);
    }
    pub fn finalize_block(&mut self, len: usize) {
        self.extend_buffer_to_len(len);
    }
}