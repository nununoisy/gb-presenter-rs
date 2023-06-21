use clap::{arg, Arg, ArgAction, value_parser, Command};
use std::path::PathBuf;
use indicatif::{FormattedDuration, HumanBytes, ProgressBar, ProgressStyle};
use std::fmt::Write;
use sameboy::{Model, Revision};
use crate::renderer::{Renderer, render_options::{RendererOptions, RenderInput, StopCondition}};

fn model_value_parser(s: &str) -> Result<Model, String> {
    match s.replace("-", "").to_lowercase().as_str() {
        "dmg" | "dmgb" => Ok(Model::DMG(Revision::RevB)),
        "cgb0" => Ok(Model::CGB(Revision::Rev0)),
        "cgba" => Ok(Model::CGB(Revision::RevA)),
        "cgbb" => Ok(Model::CGB(Revision::RevB)),
        "cgbc" => Ok(Model::CGB(Revision::RevC)),
        "cgbd" => Ok(Model::CGB(Revision::RevD)),
        "cgb" | "cgbe" => Ok(Model::CGB(Revision::RevE)),
        "mgb" => Ok(Model::MGB),
        "agb" => Ok(Model::AGB),
        _ => Err("Invalid model string".to_string())
    }
}

fn get_renderer_options() -> RendererOptions {
    let matches = Command::new("GBPresenter")
        .arg(arg!(-c --"video-codec" <CODEC> "Set the output video codec")
            .required(false)
            .default_value("libx264"))
        .arg(arg!(-C --"audio-codec" <CODEC> "Set the output audio codec")
            .required(false)
            .default_value("aac"))
        .arg(arg!(-f --"pixel-format" <FORMAT> "Set the output video pixel format")
            .required(false)
            .default_value("yuv420p"))
        .arg(arg!(-F --"sample-format" <FORMAT> "Set the output audio sample format")
            .required(false)
            .default_value("fltp"))
        .arg(arg!(-R --"sample-rate" <RATE> "Set the output audio sample rate")
            .required(false)
            .value_parser(value_parser!(i32))
            .default_value("44100"))
        .arg(arg!(-T --"track" <TRACK> "Select the 0-indexed track to play")
            .required(false)
            .value_parser(value_parser!(u8))
            .default_value("0"))
        .arg(arg!(-s --"stop-at" <CONDITION> "Set the stop condition")
            .required(false)
            .value_parser(value_parser!(StopCondition))
            .default_value("time:300"))
        .arg(arg!(-S --"stop-fadeout" <FRAMES> "Set the audio fadeout length in frames")
            .required(false)
            .value_parser(value_parser!(u64))
            .default_value("180"))
        .arg(arg!(--"ow" <WIDTH> "Set the output video width")
            .required(false)
            .value_parser(value_parser!(u32))
            .default_value("1920"))
        .arg(arg!(--"oh" <HEIGHT> "Set the output video height")
            .required(false)
            .value_parser(value_parser!(u32))
            .default_value("1080"))
        .arg(arg!(-m --"model" <MODEL> "GameBoy model to emulate")
            .required(false)
            .value_parser(model_value_parser)
            .default_value("DMG-B"))
        .arg(arg!(-g --"gbs" <GBS> "GBS file to render")
            .required(false)
            .value_parser(value_parser!(PathBuf)))
        .arg(Arg::new("lsdj")
            .short('l')
            .long("lsdj")
            .help("LSDj ROM/SAV to render")
            .required(false)
            .num_args(2)
            .value_names(["ROM", "SAV"])
            .value_parser(value_parser!(PathBuf)))
        .arg(arg!(<output> "Output video file")
            .value_parser(value_parser!(PathBuf))
            .required(true))
        .get_matches();

    let mut options = RendererOptions::default();

    if let Some(mut lsdj_files) = matches.get_many::<PathBuf>("lsdj") {
        let rom_path = lsdj_files.next().cloned().expect("ROM file argument required for --lsdj").to_str().unwrap().to_string();
        let sav_path = lsdj_files.next().cloned().expect("SAV file argument required for --lsdj").to_str().unwrap().to_string();
        options.input = RenderInput::LSDj(rom_path, sav_path);
    } else if let Some(gbs_file) = matches.get_one::<PathBuf>("gbs") {
        options.input = RenderInput::GBS(gbs_file.to_str().unwrap().to_string());
    } else {
        panic!("One of --gbs/--lsdj is required");
    }

    options.video_options.output_path = matches.get_one::<PathBuf>("output").cloned().unwrap().to_str().unwrap().to_string();
    options.video_options.video_codec = matches.get_one::<String>("video-codec").cloned().unwrap();
    options.video_options.audio_codec = matches.get_one::<String>("audio-codec").cloned().unwrap();
    options.video_options.pixel_format_out = matches.get_one::<String>("pixel-format").cloned().unwrap();
    options.video_options.sample_format_out = matches.get_one::<String>("sample-format").cloned().unwrap();

    let sample_rate = matches.get_one::<i32>("sample-rate").cloned().unwrap();
    options.video_options.sample_rate = sample_rate;
    options.video_options.audio_time_base = (1, sample_rate).into();

    options.track_index = matches.get_one::<u8>("track").cloned().unwrap();
    options.stop_condition = matches.get_one::<StopCondition>("stop-at").cloned().unwrap();
    options.fadeout_length = matches.get_one::<u64>("stop-fadeout").cloned().unwrap();

    let ow = matches.get_one::<u32>("ow").cloned().unwrap();
    let oh = matches.get_one::<u32>("oh").cloned().unwrap();
    options.video_options.resolution_out = (ow, oh);

    options.model = matches.get_one::<Model>("model").cloned().unwrap();

    // TODO: codec options

    options
}

pub fn run() {
    let options = get_renderer_options();
    let mut renderer = Renderer::new(options).unwrap();

    let pb = ProgressBar::new(0);
    let pb_style_initial = ProgressStyle::with_template("{msg}\n{spinner} Waiting for loop detection...")
        .unwrap();
    let pb_style = ProgressStyle::with_template("{msg}\n{wide_bar} {percent}%")
        .unwrap();
    pb.set_style(pb_style_initial);

    renderer.start_encoding().unwrap();
    loop {
        if !(renderer.step().unwrap()) {
            break;
        }

        if pb.length().unwrap() == 0 {
            if let Some(duration) = renderer.expected_duration_frames() {
                pb.set_length(duration as u64);
                pb.set_style(pb_style.clone());
            }
        }
        pb.set_position(renderer.current_frame());

        let current_video_duration = FormattedDuration(renderer.encoded_duration());
        let current_video_size = HumanBytes(renderer.encoded_size() as u64);
        let current_encode_rate = renderer.encode_rate();
        let song_position = match renderer.song_position() {
            Some(position) => format!("{}", position),
            None => "?".to_string()
        };
        let expected_video_duration = match renderer.expected_duration() {
            Some(duration) => FormattedDuration(duration).to_string(),
            None => "?".to_string()
        };
        let elapsed_duration = FormattedDuration(renderer.elapsed()).to_string();
        let eta_duration = match renderer.eta_duration() {
            Some(duration) => FormattedDuration(duration).to_string(),
            None => "?".to_string()
        };

        let mut message: String = "VID]".to_string();
        write!(message, " enc_time={}/{}", current_video_duration, expected_video_duration).unwrap();
        write!(message, " size={}", current_video_size).unwrap();
        write!(message, " rate={:.2}", current_encode_rate).unwrap();

        write!(message, "\nEMU]").unwrap();
        write!(message, " pos={} loop={}", song_position, renderer.loop_count()).unwrap();
        write!(message, " fps={} avg_fps={}", renderer.instantaneous_fps(), renderer.average_fps()).unwrap();

        write!(message, "\nTIM]").unwrap();
        write!(message, " run_time={}/{}", elapsed_duration, eta_duration).unwrap();

        pb.set_message(message);
    }

    pb.finish_with_message("Finalizing encode...");
    renderer.finish_encoding().unwrap();
}