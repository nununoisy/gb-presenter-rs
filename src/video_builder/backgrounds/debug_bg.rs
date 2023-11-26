use std::path::Path;
use ffmpeg_next::{format, frame};
use super::VideoBackground;

pub struct DebugBackground(u32, u32);

impl DebugBackground {
    pub fn open<P: AsRef<Path>>(path: P, width: u32, height: u32) -> Option<Self> {
        if path.as_ref().to_str().unwrap_or("") != "__debug__" {
            return None;
        }

        Some(Self(width, height))
    }
}

impl VideoBackground for DebugBackground {
    fn next_frame(&mut self) -> frame::Video {
        let mut frame = frame::Video::new(format::Pixel::RGBA, self.0, self.1);
        frame.plane_mut::<(u8, u8, u8, u8)>(0)
            .iter_mut()
            .for_each(|px| *px = (0, 0, 255, 128));
        frame
    }
}
