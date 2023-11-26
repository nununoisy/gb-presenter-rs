use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time;
use ffmpeg_next::{format, software::scaling, util::frame, media::Type, codec};
use super::VideoBackground;

fn spawn_decoding_thread(frames: Arc<Mutex<VecDeque<frame::Video>>>, path: &str, w: u32, h: u32) -> JoinHandle<()> {
    let path = path.to_string();
    thread::spawn(move || {
        println!("[MTVBG] Decoding thread started");

        let mut in_ctx = format::input(&path).unwrap();
        let in_stream = in_ctx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg_next::Error::StreamNotFound)
            .unwrap();

        let stream_idx = in_stream.index();

        let v_codec_ctx = codec::Context::from_parameters(in_stream.parameters())
            .unwrap();
        let mut v_decoder = v_codec_ctx
            .decoder()
            .video()
            .unwrap();

        let mut sws_ctx = scaling::Context::get(
            v_decoder.format(), v_decoder.width(), v_decoder.height(),
            format::Pixel::RGBA, w, h,
            scaling::Flags::FAST_BILINEAR
        ).unwrap();

        println!("[MTVBG] Starting to decode...");

        let mut decoded_frame = frame::Video::empty();
        let mut rgba_frame = frame::Video::empty();

        for (stream, packet) in in_ctx.packets() {
            if stream.index() == stream_idx {
                v_decoder.send_packet(&packet)
                    .unwrap();

                while v_decoder.receive_frame(&mut decoded_frame).is_ok() {
                    sws_ctx.run(&decoded_frame, &mut rgba_frame)
                        .unwrap();

                    {
                        let mut guarded_frames = frames.lock().unwrap();
                        guarded_frames.push_back(rgba_frame.clone());
                        if guarded_frames.len() <= 30 {
                            continue;
                        }
                    }

                    // Pause decoding if we have too many queued frames and wait for decoder
                    // to consume some before resuming so we don't gobble up RAM
                    loop {
                        {
                            let guarded_frames = frames.lock().unwrap();
                            if guarded_frames.len() <= 10 {
                                break;
                            }
                        }
                        thread::sleep(time::Duration::from_millis(100));
                    }
                }
            }
        }

        println!("[MTVBG] Decoding thread stopping");
    })
}

pub struct MTVideoBackground {
    w: u32,
    h: u32,
    handle: JoinHandle<()>,
    frames: Arc<Mutex<VecDeque<frame::Video>>>
}

impl MTVideoBackground {
    pub fn open(path: &str, w: u32, h: u32) -> Option<Self> {
        if format::input(&path).is_err() {
            return None;
        }

        let frames: Arc<Mutex<VecDeque<frame::Video>>> = Arc::new(Mutex::new(VecDeque::new()));
        let handle = spawn_decoding_thread(frames.clone(), path, w, h);

        thread::sleep(time::Duration::from_millis(50));

        Some(Self {
            w,
            h,
            handle,
            frames
        })
    }
}

impl VideoBackground for MTVideoBackground {
    fn next_frame(&mut self) -> frame::Video {
        loop {
            let mut guarded_frames = self.frames.lock().unwrap();
            if let Some(frame) = guarded_frames.pop_front() {
                break frame;
            } else {
                drop(guarded_frames);
                if self.handle.is_finished() {
                    let blank_frame = frame::Video::new(format::Pixel::RGBA, self.w, self.h);
                    break blank_frame
                }
                thread::sleep(time::Duration::from_millis(10));
            }
        }
    }
}