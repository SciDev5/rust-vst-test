use std::{array, sync::Arc};

use crossbeam::atomic::AtomicCell;
use nih_plug::params::persist::PersistentField;
use serde::{Deserialize, Serialize};

const U8_PER_U128: usize = (u128::BITS / u8::BITS) as usize;

#[derive(Serialize, Deserialize)]
pub struct TextState {
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    v: AtomicCell<[u128; TextState::L + 1]>,
}
impl Default for TextState {
    fn default() -> Self {
        Self {
            v: AtomicCell::new([0; TextState::L + 1]),
        }
    }
}

impl TextState {
    pub fn raw(&self) -> [u128; Self::L + 1] {
        self.v.load()
    }
    pub fn get_raw_hex(&self) -> String {
        let raw = self.raw();

        let mut out: [[String; U8_PER_U128]; Self::L + 1] =
            array::from_fn(|_| array::from_fn(|_| String::default()));
        for i in 0..Self::L + 1 {
            for (j, byte) in raw[i].to_le_bytes().iter().enumerate() {
                let upper = byte >> 4;
                let lower = byte & 0xf;
                out[i][j] = format!("{upper:X}{lower:X}");
            }
        }
        out.map(|v| v.join(" ")).join("\n")
    }

    pub fn len(&self) -> usize {
        self.raw()[Self::L] as usize
    }

    pub fn get_v(&self) -> String {
        Self::u128arr_to_string(self.raw())
    }
    pub fn set_v(&self, str: String) {
        self.v.store(Self::string_to_u128arr(str))
    }

    const L: usize = 256 / U8_PER_U128;

    fn u128arr_to_string(inp: [u128; Self::L + 1]) -> String {
        let v_len = inp[Self::L] as usize;

        let mut v_byte: Vec<u8> = vec![];
        'outer: for i in 0..Self::L {
            for byte in inp[i].to_le_bytes() {
                v_byte.push(byte);
                if v_byte.len() >= v_len {
                    break 'outer;
                }
            }
        }
        match String::from_utf8(v_byte) {
            Ok(v) => return v,
            _ => "".to_string(),
        }
    }
    fn string_to_u128arr(str: String) -> [u128; Self::L + 1] {
        let bytes_raw = str.bytes();
        let len = usize::min(str.bytes().len(), Self::L * U8_PER_U128);
        let mut bytes: [u8; 256] = [0; 256];
        for (i, byte) in bytes_raw.enumerate() {
            if i > bytes.len() {
                break;
            }
            bytes[i] = byte;
        }
        let mut v_u128: [u128; Self::L + 1] = [0; Self::L + 1];
        for i in 0..Self::L {
            let mut b = [0u8; U8_PER_U128];
            for j in 0..U8_PER_U128 {
                b[j] = bytes[i * U8_PER_U128 + j];
            }
            v_u128[i] = u128::from_le_bytes(b);
        }
        v_u128[Self::L] = len as u128;

        return v_u128;
    }
}

impl<'a> PersistentField<'a, TextState> for Arc<TextState> {
    fn set(&self, new_value: TextState) {
        self.v.store(new_value.v.load());
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&TextState) -> R,
    {
        f(self)
    }
}
