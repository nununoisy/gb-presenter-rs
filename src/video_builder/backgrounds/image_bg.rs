use std::path::Path;
use ffmpeg_next::{format, frame, software};
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

        let mut img_frame = frame::Video::new(format::Pixel::RGBA, w, h);
        img_frame.data_mut(0).copy_from_slice(&img.into_raw());

        let mut frame = frame::Video::new(format::Pixel::BGRA, w, h);

        let mut swc_ctx = software::converter(
            (w, h),
            format::Pixel::RGBA,
            format::Pixel::BGRA
        ).unwrap();
        swc_ctx.run(&img_frame, &mut frame).unwrap();

        Some(Self(frame))
    }
}

impl VideoBackground for ImageBackground {
    fn next_frame(&mut self) -> frame::Video {
        self.0.clone()
    }
}
