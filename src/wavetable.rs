use std::{
    fs::File,
    io::{self, Cursor, Read},
    path::Path,
};

const DEFAULT_WAVE: &[u8; 2097288] = include_bytes!("default_wave.wav");

trait Lerpable<Bound> {
    fn lerp(&self, lower: Bound, upper: Bound) -> Self;
}
impl Lerpable<f32> for f32 {
    fn lerp(&self, lower: f32, upper: f32) -> Self {
        return lower * (1.0 - self) + upper * self;
    }
}

pub struct Wavetable {
    data: Vec<Vec<f32>>,
    // data_base: Vec<f32>,
    // slice_length: usize,
}

impl Wavetable {
    pub const SIZE: usize = 2048;

    pub fn new(data_base: Vec<f32>, slice_length: usize) -> Option<Self> {
        if data_base.len() == 0 || slice_length <= 2 {
            return None;
        }
        if !data_base.len().is_power_of_two() || !slice_length.is_power_of_two() {
            return None;
        }
        if slice_length >= data_base.len() {
            return None;
        }

        let mut data = vec![vec![0.0; Self::SIZE]; Self::SIZE];
        for slice in 0..Self::SIZE {
            for sample in 0..Self::SIZE {
                data[slice][sample] = Self::sample_source(
                    &data_base,
                    slice_length,
                    sample as f32 / Self::SIZE as f32,
                    slice as f32 / Self::SIZE as f32,
                )
            }
        }

        Some(Self {
            // data_base,
            // slice_length,
            data,
        })
    }
    fn sample_source(
        data_base: &Vec<f32>,
        slice_length: usize,
        sample_phase: f32,
        slice_phase: f32,
    ) -> f32 {
        let n_slices = data_base.len() / slice_length;

        let slice_i_lw = (slice_phase * n_slices as f32) as usize;
        let slice_i_hi = (slice_i_lw + 1) % n_slices;
        let slice_k = (slice_phase * n_slices as f32) - slice_i_lw as f32;

        let sample_i_lw = (sample_phase * slice_length as f32) as usize;
        let sample_i_hi = (sample_i_lw + 1) % slice_length;
        let sample_k = (slice_phase * slice_length as f32) - sample_i_lw as f32;

        slice_k.lerp(
            sample_k.lerp(
                data_base[slice_i_lw * slice_length + sample_i_lw],
                data_base[slice_i_lw * slice_length + sample_i_hi],
            ),
            sample_k.lerp(
                data_base[slice_i_hi * slice_length + sample_i_lw],
                data_base[slice_i_hi * slice_length + sample_i_hi],
            ),
        )
    }

    pub fn sample(&self, sample_phase: f32, slice_phase: f32) -> f32 {
        self.data[((slice_phase * Self::SIZE as f32) as usize).clamp(0, Self::SIZE-1)]
            [((sample_phase * Self::SIZE as f32) as usize).clamp(0, Self::SIZE-1)]
    }
}
impl Default for Wavetable {
    fn default() -> Self {
        Self::new(Wav::from_bytes(DEFAULT_WAVE).unwrap(), 2048).expect("default wavetable invalid")
    }
}

pub struct Wav {}
impl Wav {
    pub fn from_filepath(file_path: &Path) -> Option<Vec<f32>> {
        let mut p = if let Some(p) = File::open(file_path).ok() {
            p
        } else {
            return None;
        };
        Self::from_reader(&mut p)
    }

    pub fn from_bytes<const L: usize>(bytes: &[u8; L]) -> Option<Vec<f32>> {
        Self::from_reader(&mut Cursor::new(bytes))
    }

    pub fn from_reader<R>(r: &mut R) -> Option<Vec<f32>>
    where
        R: Read + io::Seek,
    {
        let (wav_hdr, wavy) = if let Some(c) = wav::read(r).ok() {
            c
        } else {
            return None;
        };
        let data: Vec<f32> = match wav_hdr.bits_per_sample {
            8 => {
                let v = wavy.as_eight().unwrap();
                let mut data = vec![];
                for v in v {
                    data.push(*v as f32 / 255.0 * 2.0 - 1.0);
                }
                data
            }
            16 => {
                let v = wavy.as_sixteen().unwrap();
                let mut data = vec![];
                for v in v {
                    data.push(*v as f32 / 32768.0);
                }
                data
            }
            24 => {
                // who the fuck is responsible for this unholy monstrosity
                let v = wavy.as_twenty_four().unwrap();
                let mut data = vec![];
                for v in v {
                    data.push(*v as f32 / 8388608.0);
                }
                data
            }
            32 => {
                let v = wavy.as_thirty_two_float().unwrap();
                let mut data = vec![];
                for v in v {
                    data.push(*v);
                }
                data
            }
            _ => return None,
        };

        Some(data)
    }
}
