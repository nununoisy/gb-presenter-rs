use sameboy_sys::GB_gbs_info_t;
use encoding_rs::{CoderResult, SHIFT_JIS};

macro_rules! string_fn {
    ($name: tt, $offset: literal, $max_len: literal) => {
        pub fn $name(&self) -> Result<String, String> {
            self.parse_string($offset, $max_len)
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

    fn parse_string(&self, offset: usize, max_len: usize) -> Result<String, String> {
        let length = (offset..offset+max_len)
            .position(|i| self.data[i] == 0)
            .unwrap_or(max_len);

        let mut sj_decoder = SHIFT_JIS.new_decoder();
        let mut sj_result = String::new();
        sj_result.reserve(length * 4);  // Probably way more than ever needed but better safe than sorry

        let (coder_result, _bytes_read, did_replacements) = sj_decoder.decode_to_string(s, &mut result, true);
        if coder_result == CoderResult::OutputFull || did_replacements {
            // Not valid Shift-JIS, just try ASCII/Unicode
            match std::str::from_utf8(&self.data[offset..offset+length]) {
                Ok(s) => Ok(s.to_string()),
                Err(e) => Err(e.to_string())
            }
        } else {
            Ok(sj_result)
        }
    }

    string_fn!(title, 0x10, 0x20);
    string_fn!(artist, 0x30, 0x20);
    string_fn!(copyright, 0x50, 0x20);
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
