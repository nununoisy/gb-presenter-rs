use std::thread;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use crate::renderer::{Renderer, SongPosition, render_options::RendererOptions};

#[derive(Clone)]
pub struct RenderProgressInfo {
    pub frame: u64,
    pub average_fps: u32,
    pub encoded_size: usize,
    pub expected_duration_frames: Option<usize>,
    pub expected_duration: Option<Duration>,
    pub eta_duration: Option<Duration>,
    pub elapsed_duration: Duration,
    pub encoded_duration: Duration,
    pub song_position: Option<SongPosition>,
    pub loop_count: u64
}

#[derive(Clone)]
pub enum RenderThreadMessage {
    Error(String),
    RenderStarting,
    RenderProgress(RenderProgressInfo),
    RenderComplete
}

macro_rules! rt_unwrap {
    ($v: expr, $cb: tt) => {
        match $v {
            Ok(v) => v,
            Err(e) => {
                $cb(RenderThreadMessage::Error(e));
                return;
            }
        }
    };
}

pub fn render_thread<F>(cb: F) -> (thread::JoinHandle<()>, mpsc::Sender<Option<RendererOptions>>)
    where
        F: Fn(RenderThreadMessage) + Send + 'static
{
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        println!("Renderer thread started");

        loop {
            let options = match rx.recv().unwrap() {
                Some(o) => o,
                None => return
            };
            cb(RenderThreadMessage::RenderStarting);

            let mut renderer = rt_unwrap!(Renderer::new(options), cb);
            rt_unwrap!(renderer.start_encoding(), cb);

            let mut last_progress_timestamp = Instant::now();
            // Janky way to force an update
            last_progress_timestamp.checked_sub(Duration::from_secs(2));

            loop {
                match rx.try_recv() {
                    Ok(None) => return,
                    _ => ()
                }
                if !(rt_unwrap!(renderer.step(), cb)) {
                    break;
                }

                if last_progress_timestamp.elapsed().as_secs_f64() >= 0.5 {
                    last_progress_timestamp = Instant::now();

                    let progress_info = RenderProgressInfo {
                        frame: renderer.current_frame(),
                        average_fps: renderer.average_fps(),
                        encoded_size: renderer.encoded_size(),
                        expected_duration_frames: renderer.expected_duration_frames(),
                        expected_duration: renderer.expected_duration(),
                        eta_duration: renderer.eta_duration(),
                        elapsed_duration: renderer.elapsed(),
                        encoded_duration: renderer.encoded_duration(),
                        song_position: renderer.song_position(),
                        loop_count: renderer.loop_count()
                    };

                    cb(RenderThreadMessage::RenderProgress(progress_info));
                }
            }

            rt_unwrap!(renderer.finish_encoding(), cb);
            cb(RenderThreadMessage::RenderComplete);
        }
    });
    (handle, tx)
}