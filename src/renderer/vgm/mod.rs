mod vgm;
mod gd3;
pub mod converter;

pub use vgm::{Vgm, VgmIterItem};
pub use gd3::{Gd3};

pub fn duration_frames(vgm: &Vgm, loops: usize) -> usize {
    let loop_samples = vgm.loop_sample_count() as f32;
    let intro_samples = (vgm.sample_count() as f32) - loop_samples;

    let loop_frames = ((60.0 * loop_samples) / 44100.0).round() as usize;
    let intro_frames = ((60.0 * intro_samples) / 44100.0).round() as usize;

    intro_frames + (loops * loop_frames)
}
