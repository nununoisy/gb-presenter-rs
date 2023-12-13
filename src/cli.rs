use clap::{arg, Arg, ArgAction, value_parser, Command};
use csscolorparser::Color as CssColor;
use std::path::PathBuf;
use indicatif::{FormattedDuration, HumanBytes, ProgressBar, ProgressStyle};
use std::fmt::Write;
use std::fs;
use sameboy::{Model, Revision};
use tiny_skia::Color;
use crate::config::Config;
use crate::renderer::{Renderer, render_options::{RendererOptions, RenderInput, StopCondition}, vgm};

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

fn color_value_parser(s: &str) -> Result<Color, String> {
    let parsed_color = s.parse::<CssColor>()
        .map_err(|e| e.to_string())?;

    Ok(Color::from_rgba(
        parsed_color.r as f32,
        parsed_color.g as f32,
        parsed_color.b as f32,
        parsed_color.a as f32
    ).unwrap())
}


fn codec_option_value_parser(s: &str) -> Result<(String, String), String> {
    let (key, value) = s.split_once('=')
        .ok_or("Invalid option specification (must be of the form 'option=value').".to_string())?;

    Ok((key.to_string(), value.to_string()))
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
        .arg(arg!(-T --"track" <TRACK> "Select the 1-indexed track to play")
            .required(false)
            .value_parser(value_parser!(u8))
            .default_value("1")
            .action(ArgAction::Append))
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
        .arg(arg!(-o --"video-option" <OPTION> "Pass an option to the video codec (option=value)")
            .required(false)
            .value_parser(codec_option_value_parser)
            .action(ArgAction::Append))
        .arg(arg!(-O --"audio-option" <OPTION> "Pass an option to the audio codec (option=value)")
            .required(false)
            .value_parser(codec_option_value_parser)
            .action(ArgAction::Append))
        .arg(arg!(-m --"model" <MODEL> "GameBoy model to emulate")
            .required(false)
            .value_parser(model_value_parser)
            .default_value("CGB-E"))
        .arg(arg!(-k --"channel-color" "Set the colors for a channel.")
            .required(false)
            .num_args(3..=18)
            .value_names(&["CHIP", "CHANNEL", "COLORS..."])
            .action(ArgAction::Append))
        .arg(arg!(-H --"hide-channel" "Hide a channel from the visualization.")
            .required(false)
            .num_args(2)
            .value_names(&["CHIP", "CHANNEL"])
            .action(ArgAction::Append))
        .arg(arg!(-i --"import-config" <CONFIGFILE> "Import configuration from a RusticNES TOML file.")
            .value_parser(value_parser!(PathBuf))
            .required(false))
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
        .arg(Arg::new("2xlsdj")
            .short('2')
            .long("2xlsdj")
            .help("2x LSDj ROM/SAV pair to render")
            .required(false)
            .num_args(4)
            .value_names(["ROM", "SAV", "ROM2X", "SAV2X"])
            .value_parser(value_parser!(PathBuf)))
        .arg(arg!(-v --"vgm" <VGM> "VGM file to render")
            .required(false)
            .value_parser(value_parser!(PathBuf)))
        .arg(arg!(<output> "Output video file")
            .value_parser(value_parser!(PathBuf))
            .required(true))
        .get_matches();

    let mut options = RendererOptions::default();

    options.auto_lsdj_sync = true;

    if let Some(mut lsdj_files) = matches.get_many::<PathBuf>("lsdj") {
        let rom_path = lsdj_files.next().cloned().expect("ROM file argument required for --lsdj").to_str().unwrap().to_string();
        let sav_path = lsdj_files.next().cloned().expect("SAV file argument required for --lsdj").to_str().unwrap().to_string();
        options.input = RenderInput::LSDj(rom_path, sav_path);
    } else if let Some(mut lsdj_files) = matches.get_many::<PathBuf>("2xlsdj") {
        let rom_path = lsdj_files.next().cloned().expect("ROM file argument required for --2xlsdj").to_str().unwrap().to_string();
        let sav_path = lsdj_files.next().cloned().expect("SAV file argument required for --2xlsdj").to_str().unwrap().to_string();
        let rom_path_2x = lsdj_files.next().cloned().expect("ROM2X file argument required for --2xlsdj").to_str().unwrap().to_string();
        let sav_path_2x = lsdj_files.next().cloned().expect("SAV2X file argument required for --2xlsdj").to_str().unwrap().to_string();
        options.input = RenderInput::LSDj2x(rom_path, sav_path, rom_path_2x, sav_path_2x);
    } else if let Some(gbs_file) = matches.get_one::<PathBuf>("gbs") {
        options.input = RenderInput::GBS(gbs_file.to_str().unwrap().to_string());
    } else if let Some(vgm_file) = matches.get_one::<PathBuf>("vgm") {
        options.input = RenderInput::VGM(vgm_file.to_str().unwrap().to_string(), 60, 0);
    } else {
        panic!("One of --gbs/--lsdj/--2xlsdj/--vgm is required");
    }

    options.video_options.output_path = matches.get_one::<PathBuf>("output").cloned().unwrap().to_str().unwrap().to_string();
    options.video_options.video_codec = matches.get_one::<String>("video-codec").cloned().unwrap();
    options.video_options.audio_codec = matches.get_one::<String>("audio-codec").cloned().unwrap();
    options.video_options.pixel_format_out = matches.get_one::<String>("pixel-format").cloned().unwrap();
    options.video_options.sample_format_out = matches.get_one::<String>("sample-format").cloned().unwrap();

    if options.video_options.output_path.ends_with(".mov") {
        // Fairly close approximation of the Game Boy's frame rate with a timebase denominator <100000.
        // Required to avoid "codec timebase is very high" warning from the QuickTime encoder.
        options.video_options.video_time_base = (1_097, 65_536).into();
    }

    let sample_rate = matches.get_one::<i32>("sample-rate").cloned().unwrap();
    options.video_options.sample_rate = sample_rate;
    options.video_options.audio_time_base = (1, sample_rate).into();

    for (i, track) in matches.get_many::<u8>("track").unwrap().cloned().enumerate() {
        match i {
            0 => options.track_index = track.saturating_sub(1),
            1 => options.track_index_2x = track.saturating_sub(1),
            _ => panic!("Too many arguments for --track")
        };
    }

    options.stop_condition = match (matches.get_one::<StopCondition>("stop-at").cloned().unwrap(), &options.input) {
        (StopCondition::Loops(loops), RenderInput::VGM(vgm_path, engine_rate, _)) => {
            let vgm_s = vgm::Vgm::open(vgm_path).unwrap();
            let frames = vgm::duration_frames(&vgm_s, *engine_rate, loops);
            StopCondition::Frames(frames as u64)
        },
        (stop_condition, _) => stop_condition
    };

    options.fadeout_length = matches.get_one::<u64>("stop-fadeout").cloned().unwrap();

    let ow = matches.get_one::<u32>("ow").cloned().unwrap();
    let oh = matches.get_one::<u32>("oh").cloned().unwrap();
    options.set_resolution_smart(ow, oh);

    options.model = matches.get_one::<Model>("model").cloned().unwrap();

    if let Some(video_options) = matches.get_many::<(String, String)>("video-option") {
        for (k, v) in video_options.cloned() {
            options.video_options.video_codec_params.insert(k, v);
        }
    }
    if let Some(audio_options) = matches.get_many::<(String, String)>("audio-option") {
        for (k, v) in audio_options.cloned() {
            options.video_options.audio_codec_params.insert(k, v);
        }
    }

    options.config = match matches.get_one::<PathBuf>("import-config") {
        Some(config_path) => {
            let config = fs::read_to_string(config_path).expect("Failed to read config file!");
            Config::from_toml(&config).expect("Failed to parse config file!")
        },
        None => Config::default()
    };

    if let Some(channel_settings) = matches.get_occurrences::<String>("channel-color") {
        for channel_setting_parts in channel_settings.map(Iterator::collect::<Vec<&String>>) {
            let chip = channel_setting_parts
                .get(0)
                .expect("Channel setting must have chip name");
            let channel = channel_setting_parts
                .get(1)
                .expect("Channel setting must have channel name");

            let setting = options.config.piano_roll.settings.settings_mut_by_name(chip.as_str(), channel.as_str())
                .expect(format!("Unknown chip/channel specified: {} {}", chip, channel).as_str());

            if setting.colors().len() != channel_setting_parts.len() - 2 {
                panic!("Wrong number of colors specified for chip/channel {} {}: expected {} colors", chip, channel, setting.colors().len());
            }

            let new_colors: Vec<Color> = channel_setting_parts.iter()
                .skip(2)
                .map(|c| color_value_parser(c.as_str()).expect("Invalid color"))
                .collect();
            setting.set_colors(&new_colors);
        }

        if let Some(hidden_channels) = matches.get_occurrences::<String>("hide-channel") {
            for hidden_channel_parts in hidden_channels.map(Iterator::collect::<Vec<&String>>) {
                let chip = hidden_channel_parts
                    .get(0)
                    .expect("Hidden channel must have chip name");
                let channel = hidden_channel_parts
                    .get(1)
                    .expect("Hidden channel must have channel name");

                options.config.piano_roll.settings.settings_mut_by_name(chip.as_str(), channel.as_str())
                    .expect(format!("Unknown chip/channel specified: {} {}", chip, channel).as_str())
                    .set_hidden(true);
            }
        }
    }

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
            Some(position) => position.to_string(),
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

        let mut message = String::with_capacity(300);
        write!(message, "VID]").unwrap();
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