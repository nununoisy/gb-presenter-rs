use std::env;

mod visualizer;
mod video_builder;
mod renderer;
mod gui;
mod cli;

fn main() {
    video_builder::init().unwrap();

    match env::args().len() {
        1 => gui::run(),
        _ => cli::run()
    };
}
