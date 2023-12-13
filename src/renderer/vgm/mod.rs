mod vgm;
mod gd3;
pub mod converter;

pub use vgm::Vgm;

pub fn duration_frames(vgm: &Vgm, engine_rate: u32, loops: usize) -> usize {
    let loop_samples = vgm.loop_sample_count();
    let intro_samples = vgm.sample_count() - loop_samples;

    let loop_frames = converter::samples_to_frames(loop_samples, engine_rate) as usize;
    let intro_frames = converter::samples_to_frames(intro_samples, engine_rate) as usize;

    intro_frames + (loops * loop_frames)
}
