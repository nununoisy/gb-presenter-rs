mod debug_bg;
mod video_bg;
mod image_bg;

use std::path::Path;
use ffmpeg_next::frame;

pub trait VideoBackground {
    fn next_frame(&mut self) -> frame::Video;
}

pub fn get_video_background<P: AsRef<Path>>(path: P, width: u32, height: u32) -> Option<Box<dyn VideoBackground>> {
    if let Some(debug_vbg) = debug_bg::DebugBackground::open(&path, width, height) {
        return Some(Box::new(debug_vbg));
    }

    // Use FFmpeg for GIFs
    if !path.as_ref().to_str().unwrap_or("").ends_with(".gif") {
        if let Some(image_vbg) = image_bg::ImageBackground::open(&path, width, height) {
            return Some(Box::new(image_vbg));
        }
    }

    if let Some(video_vbg) = video_bg::MTVideoBackground::open(path.as_ref().to_str().unwrap_or(""), width, height) {
        return Some(Box::new(video_vbg));
    }

    None
}
