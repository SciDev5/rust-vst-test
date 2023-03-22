use nih_plug::prelude::{Smoother, SmoothingStyle};

use crate::util::param_range::ParamRange;

pub struct InputFrequencyParam {
    midi_note: u8,
    pub pitchbend: InputParam,
    // modulation_oct: f32,
    buffer: Vec<f32>,
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
            buffer: vec![],
        }
    }
    pub fn prepare(&mut self) {
        self.buffer = self.pitchbend.buffer.iter().map(|v| nih_plug::util::f32_midi_note_to_freq(v + self.midi_note as f32)).collect()
    }
    pub fn get(&self) -> &Vec<f32> {
        &self.buffer
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
        for _ in self.buffer.len() .. sample_id {
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


pub enum ParamPolarity {
    Monopolar,
    Bipolar,
}
impl ParamPolarity {
    pub fn convert(
        self,
        buffer: &Vec<f32>,
        to: ParamPolarity,
    ) -> Vec<f32> {
        match self {
            Self::Monopolar => match to {
                Self::Monopolar => buffer.clone(),
                Self::Bipolar => buffer.iter().map(|v| (*v) * 2.0 - 1.0).collect(),
            },
            Self::Bipolar => match to {
                Self::Monopolar => buffer.iter().map(|v| (*v) * 0.5 + 0.5).collect(),
                Self::Bipolar => buffer.clone(),
            },
        }
    }
    // pub fn convert_single(
    //     self,
    //     v: f32,
    //     to: ParamPolarity,
    // ) -> f32 {
    //     match self {
    //         Self::Monopolar => match to {
    //             Self::Monopolar => v,
    //             Self::Bipolar => v * 2.0 - 1.0,
    //         },
    //         Self::Bipolar => match to {
    //             Self::Monopolar => v * 0.5 + 0.5,
    //             Self::Bipolar => v,
    //         },
    //     }
    // }
}
pub trait ParamSource {
    const POLARITY: ParamPolarity;
    fn source_param_buffer(&self) -> &Vec<f32>;
}
pub trait ParamSourceImpl {
    fn get_param_buffer(&self, to_polarity: ParamPolarity) -> Vec<f32>;
}


impl <T> ParamSourceImpl for T where T : ParamSource {
    fn get_param_buffer(&self, to_polarity: ParamPolarity) -> Vec<f32> {
        T::POLARITY.convert(self.source_param_buffer(), to_polarity)
    }
}



pub struct ParamImmut {
    value_base: f32,
    range: ParamRange,
    value_off: f32,
    value: f32,
}
impl ParamImmut {
    pub fn new(
        value_base: f32,
        range: ParamRange,
    ) -> Self {
        Self {
            value_base,
            range,
            value_off: 0.0,
            value: value_base,
        }
    }

    fn update_value(&mut self) {
        self.value = self.range.denormalize(self.range.normalize(self.value_base) + self.value_off);
    }
    pub fn rebase(&mut self, value_base: f32) {
        self.value_base = value_base;
        self.update_value()
    }
    pub fn send_paraminit(&mut self, v: f32, mag: f32) {
        self.value_off += v * mag;
        self.update_value()
    }
    pub fn read(&self) -> f32 {
        self.value
    }
}

pub struct Param {
    value_base: f32,
    range: ParamRange,
    value_off_buffer: Vec<f32>,
    value_keytrack_buffer: Vec<f32>,
    pub value: Vec<f32>,
}
impl Param {
    pub fn new(
        value_base: f32,
        range: ParamRange,
    ) -> Self {
        Self {
            value_base,
            range,
            value_off_buffer: vec![],
            value_keytrack_buffer: vec![],
            value: vec![],
        }
    }

    fn value_off_buffer_initialized(&self, block_len: usize) -> bool {
        self.value_off_buffer.len() == block_len
    }
    fn value_keytrack_buffer_initialized(&self, block_len: usize) -> bool {
        self.value_keytrack_buffer.len() == block_len
    }

    pub fn rebase(&mut self, value_base: f32) {
        self.value_base = value_base;
    }

    pub fn send<T : ParamSource>(&mut self, param: &T, polarity: ParamPolarity, mag: f32) {
        let param_data = param.get_param_buffer(polarity);
        let block_len = param_data.len();
        if !self.value_off_buffer_initialized(block_len) {
            self.value_off_buffer.clear();
            self.value_off_buffer.resize(block_len, 0.0);
        }

        for i in 0 .. block_len {
            self.value_off_buffer[i] += mag * param_data[i];
        }
    }
    pub fn send_key_track(&mut self, freq: &InputFrequencyParam) {
        self.send_key_track_withmag(freq, 1.0);
    }
    pub fn send_key_track_withmag(&mut self, freq: &InputFrequencyParam, mag: f32) {
        let freq_data = freq.get();
        let block_len = freq_data.len();

        if !self.value_keytrack_buffer_initialized(block_len) {
            self.value_keytrack_buffer.clear();
            self.value_keytrack_buffer.resize(block_len, 0.0);
        }

        for i in 0 .. block_len {
            self.value_keytrack_buffer[i] = freq_data[i] * mag;
        }
    }

    /// Take the values out of `value_off_buffer` and send them.
    pub fn take(&mut self, block_len: usize) -> &Vec<f32> {
        self.value.clear();
        let use_off = self.value_off_buffer_initialized(block_len);
        let use_keytrack = self.value_keytrack_buffer_initialized(block_len);
        if use_off {
            if use_keytrack {
                for i in 0 .. block_len {
                    self.value.push(self.range.denormalize(self.value_off_buffer[i] + self.range.normalize(
                        self.value_base + self.value_keytrack_buffer[i]
                    )));
                }
            } else {
                for i in 0 .. block_len {
                    self.value.push(self.range.denormalize(self.value_off_buffer[i] + self.range.normalize(
                        self.value_base
                    )));
                }
            }
        } else {
            if use_keytrack {
                for i in 0 .. block_len {
                    self.value.push(
                        self.value_base + self.value_keytrack_buffer[i]
                    );
                }
            } else {
                self.value.resize(block_len, self.value_base);
            }
        }
        self.value_off_buffer.clear();
        self.value_keytrack_buffer.clear();
        &self.value
    }
}