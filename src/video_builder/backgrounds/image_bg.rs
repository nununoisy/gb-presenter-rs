use std::path::Path;
use ffmpeg_next::{format, frame};
use image;
use crate::video_builder::backgrounds::VideoBackground;

pub struct ImageBackground(frame::Video);

impl ImageBackground {
    pub fn open<P: AsRef<Path>>(path: P, w: u32, h: u32) -> Option<Self> {
        let dyn_img = match image::open(path) {
            Ok(i) => i,
            Err(_) => return None
        };
        let img = image::imageops::resize(&dyn_img.to_rgba8(), w, h, image::imageops::Gaussian);

        let mut frame = frame::Video::new(format::Pixel::RGBA, w, h);
        frame.data_mut(0).copy_from_slice(&img.into_raw());

        Some(Self(frame))
    }
}

impl VideoBackground for ImageBackground {
    fn next_frame(&mut self) -> frame::Video {
        self.0.clone()
    }
}
