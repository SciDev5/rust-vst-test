use crate::{util::Lerpable};
use std::{
    fs::File,
    io::{self, Cursor, Read},
    path::Path,
};

const DEFAULT_WAVE: &[u8; 2097288] = include_bytes!("./assets/default_wave.wav");

fn remap_index(x: f32, size: usize) -> usize {
    ((x * size as f32) as usize).min(size - 1)
}

pub struct WavetableRaw {
    data: Box<[f32; Self::SIZE * Self::SIZE]>,
}
impl WavetableRaw {
    pub const SIZE: usize = 2048;

    pub fn sample(&self, sample: f32, slice: f32) -> f32 {
        self.data[Self::index(
            remap_index(slice, Self::SIZE),
            remap_index(sample, Self::SIZE),
        )]
    }

    fn index(slice: usize, sample: usize) -> usize {
        slice * Self::SIZE + sample
    }
}
impl Default for WavetableRaw {
    fn default() -> Self {
        Self {
            data: vec![0.0f32; Self::SIZE * Self::SIZE].try_into().unwrap(),
        }
    }
}

pub type WavetableSlices = Vec<[f32; Wavetable::SLICE_LEN]>;
pub struct Wavetable {
    pub data: WavetableRaw,

    slices: WavetableSlices,
}

impl Wavetable {
    pub const SLICE_LEN: usize = 512;

    pub fn slice_downsample(data_raw: &Vec<f32>, real_slice_len: usize) -> Option<Self> {
        if real_slice_len % Self::SLICE_LEN != 0 || real_slice_len < Self::SLICE_LEN {
            return None;
        }
        let downsample_ratio = real_slice_len / Self::SLICE_LEN;
        let mut downsampled_data = vec![0.0f32; data_raw.len() / downsample_ratio];
        for i in 0..downsampled_data.len() {
            let mut sum = 0.0f32;
            for sample in data_raw[downsample_ratio * i..downsample_ratio * (i + 1)].into_iter() {
                sum += sample;
            }
            downsampled_data[i] = sum / downsample_ratio as f32;
        }
        Self::slice(&downsampled_data)
    }
    pub fn slice(data_raw: &Vec<f32>) -> Option<Self> {
        if data_raw.len() % Self::SLICE_LEN != 0 {
            return None;
        }
        let mut slices: WavetableSlices =
            vec![[0.0; Wavetable::SLICE_LEN]; data_raw.len() / Self::SLICE_LEN];
        for slice_id in 0..slices.len() {
            for sample_id in 0..Wavetable::SLICE_LEN {
                slices[slice_id][sample_id] = data_raw[sample_id + slice_id * Wavetable::SLICE_LEN];
            }
        }
        Some(Self::new(slices))
    }
    pub fn new(slices: WavetableSlices) -> Self {
        if slices.len() == 0 {
            return Self {
                data: WavetableRaw::default(),
                slices: vec![[0.0; Self::SLICE_LEN]],
            };
        }

        let mut instance = Self {
            data: WavetableRaw::default(),
            slices,
        };
        instance.update_data();

        return instance;
    }
    pub fn update_data(&mut self) {
        let mut slices: Vec<[f32; WavetableRaw::SIZE]> = vec![];
        for slice in &self.slices {
            slices.push(Self::upsample_slice(slice));
        }

        let slices_interpolated = Self::interp_slices(slices);

        for slice in 0..WavetableRaw::SIZE {
            for sample in 0..WavetableRaw::SIZE {
                self.data.data[WavetableRaw::index(slice, sample)] =
                    slices_interpolated[slice][sample];
            }
        }
    }
    fn upsample_slice(slice: &[f32; Wavetable::SLICE_LEN]) -> [f32; WavetableRaw::SIZE] {
        Self::upsample_slice_linear(slice)
    }
    fn interp_slices(slices: Vec<[f32; WavetableRaw::SIZE]>) -> Vec<[f32; WavetableRaw::SIZE]> {
        Self::interp_slices_linear(slices)
    }

    fn upsample_slice_linear(slice: &[f32; Wavetable::SLICE_LEN]) -> [f32; WavetableRaw::SIZE] {
        let mut out = [0.0f32; WavetableRaw::SIZE];
        for i in 0..Wavetable::SLICE_LEN {
            const R: usize = WavetableRaw::SIZE / Wavetable::SLICE_LEN;
            let i1 = (i + 1) % Wavetable::SLICE_LEN;
            for j in 0..R {
                let k = j as f32 / R as f32;
                out[i * R + j] = k.lerp(slice[i], slice[i1]);
            }
        }
        return out;
    }
    fn interp_slices_linear(
        slices: Vec<[f32; WavetableRaw::SIZE]>,
    ) -> Vec<[f32; WavetableRaw::SIZE]> {

        let mut slices_out: Vec<[f32; WavetableRaw::SIZE]> = Vec::with_capacity(WavetableRaw::SIZE);

        let mut out = [0.0f32; WavetableRaw::SIZE];
        for slice_n in 0..WavetableRaw::SIZE {
            let table_k = slice_n as f32 / (WavetableRaw::SIZE - 1) as f32;
            let j_f32 = table_k * (slices.len() - 1) as f32;
            let j = (j_f32 as usize).min(slices.len() - 2);
            let k = j as f32 - j_f32;

            let slice0 = slices[j];
            let slice1 = slices[j + 1];
            for i in 0..WavetableRaw::SIZE {
                out[i] = k.lerp(slice0[i], slice1[i]);
            }
            slices_out.push(out);
        }
        return slices_out;
    }
    
    // fn interp_slices_fftmorph(
    //     slices: Vec<[f32; WavetableRaw::SIZE]>,
    // ) -> Vec<[f32; WavetableRaw::SIZE]> {
    //     let planner = FftPlanner::<f32>::new();
    //     let mut fft_fwd = planner.plan_fft_forward(WavetableRaw::SIZE);
    //     let mut fft_inv = planner.plan_fft_inverse(WavetableRaw::SIZE);
    //     // todo
    // }
}
impl Default for Wavetable {
    fn default() -> Self {
        Self::slice_downsample(&Wav::from_bytes(DEFAULT_WAVE).unwrap(), 2048).unwrap()
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
