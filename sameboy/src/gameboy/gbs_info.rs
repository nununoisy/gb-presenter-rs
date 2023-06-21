use sameboy_sys::GB_gbs_info_t;
use std::slice;
use encoding_rs::{CoderResult, SHIFT_JIS};

macro_rules! string_fn {
    ($name: tt, $field: tt) => {
        pub fn $name(&self) -> Result<String, String> {
            let data = unsafe {
                let s = (*self.as_ptr()).$field.as_slice();
                slice::from_raw_parts(s.as_ptr() as *const u8, s.len())
            };

            Self::parse_string(data)
        }
    }
}

pub struct GbsInfo {
    ptr: *mut GB_gbs_info_t,
    is_owned: bool
}

impl GbsInfo {
    pub fn new() -> Self {
        unsafe {
            let inner = Box::new(GB_gbs_info_t {
                track_count: 0,
                first_track: 0,
                title: [0; 33],
                author: [0; 33],
                copyright: [0; 33],
            });

            Self {
                ptr: Box::into_raw(inner),
                is_owned: true
            }
        }
    }

    pub unsafe fn wrap(ptr: *mut GB_gbs_info_t) -> Self {
        Self {
            ptr,
            is_owned: false
        }
    }

    pub unsafe fn as_ptr(&self) -> *const GB_gbs_info_t {
        self.ptr as *const _
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut GB_gbs_info_t {
        self.ptr
    }

    pub fn track_count(&self) -> u8 {
        unsafe {
            (*self.as_ptr()).track_count
        }
    }

    pub fn first_track(&self) -> u8 {
        unsafe {
            (*self.as_ptr()).first_track
        }
    }

    fn parse_string(data: &[u8]) -> Result<String, String> {
        let length = data.iter()
            .position(|&b| b == 0)
            .unwrap_or(data.len());

        let mut sj_decoder = SHIFT_JIS.new_decoder();
        let mut sj_result = String::new();
        sj_result.reserve(length * 4);  // Probably way more than ever needed but better safe than sorry

        let (coder_result, _bytes_read, did_replacements) = sj_decoder.decode_to_string(&data[0..length], &mut sj_result, true);
        if coder_result == CoderResult::OutputFull || did_replacements {
            // Not valid Shift-JIS, just try ASCII/Unicode
            match std::str::from_utf8(&data[0..length]) {
                Ok(s) => Ok(s.to_string()),
                Err(e) => Err(e.to_string())
            }
        } else {
            Ok(sj_result)
        }
    }

    string_fn!(title, title);
    string_fn!(artist, author);
    string_fn!(copyright, copyright);
}

impl Drop for GbsInfo {
    fn drop(&mut self) {
        unsafe {
            if self.is_owned {
                drop(Box::from_raw(self.ptr))
            }
        }
    }
}
