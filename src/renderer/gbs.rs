use std::fs;
use std::path::Path;
use encoding_rs::{CoderResult, SHIFT_JIS};

pub fn decode_shift_jis(s: &[u8]) -> Option<String> {
    let mut decoder = SHIFT_JIS.new_decoder();
    let mut result = String::new();
    result.reserve(s.len() * 4);  // Probably way more than ever needed but better safe than sorry

    let (coder_result, _bytes_read, did_replacements) = decoder.decode_to_string(s, &mut result, true);
    if coder_result == CoderResult::OutputFull || did_replacements {
        return None;
    }

    Some(result)
}

macro_rules! string_fn {
    ($name: tt, $offset: literal, $max_len: literal) => {
        pub fn $name(&self) -> Result<String, String> {
            self.parse_string($offset, $max_len)
        }
    }
}

pub struct Gbs {
    data: Vec<u8>
}

impl Gbs {
    pub fn new(data: &[u8]) -> Result<Self, String> {
        if &data[0..4] != b"GBS\x01" {
            return Err("Invalid GBS file".to_string());
        }

        Ok(Self {
            data: data.to_vec()
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| e.to_string())?;
        Self::new(&data)
    }

    pub fn song_count(&self) -> u8 {
        self.data[4]
    }

    pub fn starting_song(&self) -> u8 {
        self.data[5]
    }

    fn parse_string(&self, offset: usize, max_len: usize) -> Result<String, String> {
        let end = (offset..offset+max_len)
            .position(|i| self.data[i] == 0)
            .unwrap_or(max_len);

        if let Some(shift_jis) = decode_shift_jis(&self.data[offset..offset+end]) {
            return Ok(shift_jis);
        }
        match std::str::from_utf8(&self.data[offset..offset+end]) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(e.to_string())
        }
    }

    string_fn!(title, 0x10, 0x20);
    string_fn!(artist, 0x30, 0x20);
    string_fn!(copyright, 0x50, 0x20);
}
