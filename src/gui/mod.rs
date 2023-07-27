mod render_thread;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path;
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;
use indicatif::{FormattedDuration, HumanBytes};
use native_dialog::{FileDialog, MessageDialog, MessageType};
use slint;
use slint::{Color, Model as _};
use sameboy::{ApuChannel, Model, Revision};
use crate::gui::render_thread::{RenderThreadMessage, RenderThreadRequest};
use crate::renderer::gbs::Gbs;
use crate::renderer::{lsdj, vgm};
use crate::renderer::render_options::{RendererOptions, RenderInput, StopCondition};
use crate::video_builder::backgrounds::VideoBackground;
use crate::visualizer::channel_settings::{ChannelSettingsManager, ChannelSettings};

slint::include_modules!();

// The return type looks wrong but it is not
fn slint_string_arr<I>(a: I) -> slint::ModelRc<slint::SharedString>
    where
        I: IntoIterator<Item = String>
{
    let shared_string_vec: Vec<slint::SharedString> = a.into_iter()
        .map(|s| s.into())
        .collect();
    slint::ModelRc::new(slint::VecModel::from(shared_string_vec))
}

fn slint_int_arr<I, N>(a: I) -> slint::ModelRc<i32>
    where
        N: Into<i32>,
        I: IntoIterator<Item = N>
{
    let int_vec: Vec<i32> = a.into_iter()
        .map(|n| n.into())
        .collect();
    slint::ModelRc::new(slint::VecModel::from(int_vec))
}

fn slint_color_component_arr<I: IntoIterator<Item = raqote::Color>>(a: I) -> slint::ModelRc<slint::ModelRc<i32>> {
    let color_vecs: Vec<slint::ModelRc<i32>> = a.into_iter()
        .map(|c| slint::ModelRc::new(slint::VecModel::from(vec![c.r() as i32, c.g() as i32, c.b() as i32])))
        .collect();
    slint::ModelRc::new(slint::VecModel::from(color_vecs))
}

fn get_default_channel_settings() -> HashMap<(String, String), ChannelSettings> {
    let manager = ChannelSettingsManager::default();
    let mut result: HashMap<(String, String), ChannelSettings> = HashMap::new();

    result.insert(("LR35902".to_string(), "Pulse 1".to_string()), manager.settings(ApuChannel::Pulse1));
    result.insert(("LR35902".to_string(), "Pulse 2".to_string()), manager.settings(ApuChannel::Pulse2));
    result.insert(("LR35902".to_string(), "Wave".to_string()), manager.settings(ApuChannel::Wave));
    result.insert(("LR35902".to_string(), "Noise".to_string()), manager.settings(ApuChannel::Noise));

    result
}

fn browse_for_rom_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["gb", "gbs", "vgm"])
        .add_filter("LSDj ROMs", &["gb"])
        .add_filter("GameBoy Sound Files", &["gbs"])
        .add_filter("Furnace/DefleMask VGMs", &["vgm"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_sav_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("LSDj Saves", &["sav"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_background_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["mp4", "mkv", "mov", "avi", "webm", "gif", "jpg", "jpeg", "png", "bmp", "tif", "tiff", "webp", "qoi"])
        .add_filter("Video background formats", &["mp4", "mkv", "mov", "avi", "webm", "gif"])
        .add_filter("Image background formats", &["jpg", "jpeg", "png", "bmp", "tif", "tiff", "webp", "qoi"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_video_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["mp4", "mkv", "mov"])
        .add_filter("MPEG-4 Video", &["mp4"])
        .add_filter("Matroska Video", &["mkv"])
        .add_filter("QuickTime Video", &["mov"])
        .show_save_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn confirm_prores_export_dialog() -> bool {
    MessageDialog::new()
        .set_title("GBPresenter")
        .set_text("You have chosen to export a QuickTime video. Do you want to export in ProRes 4444 format to \
                   preserve alpha information for video editing? Note that ProRes 4444 is a lossless codec, so \
                   the exported file may be very large.")
        .set_type(MessageType::Info)
        .show_confirm()
        .unwrap()
}

fn display_error_dialog(text: &str) {
    MessageDialog::new()
        .set_title("GBPresenter")
        .set_text(text)
        .set_type(MessageType::Error)
        .show_alert()
        .unwrap();
}

pub fn run() {
    let main_window = MainWindow::new().unwrap();

    main_window.global::<ColorUtils>().on_hex_to_color(|hex| {
        let rgb = u32::from_str_radix(hex.to_string().trim_start_matches("#"), 16).unwrap_or(0);

        Color::from_argb_encoded(0xFF000000 | rgb)
    });

    main_window.global::<ColorUtils>().on_color_to_hex(|color| {
        format!("#{:02x}{:02x}{:02x}", color.red(), color.green(), color.blue()).into()
    });

    main_window.global::<ColorUtils>().on_color_components(|color| {
        slint_int_arr([color.red() as i32, color.green() as i32, color.blue() as i32])
    });

    let channel_settings = get_default_channel_settings();
    for ((chip, channel), settings) in channel_settings.iter() {
        let configs_model = match chip.as_str() {
            "LR35902" => main_window.get_config_lr35902(),
            _ => continue
        };
        let mut configs: Vec<ChannelConfig> = configs_model
            .as_any()
            .downcast_ref::<slint::VecModel<ChannelConfig>>()
            .unwrap()
            .iter()
            .collect();

        if let Some(config) = configs.iter_mut().find(|cfg| channel.clone() == cfg.name.to_string()) {
            config.hidden = settings.hidden();
            config.colors = slint_color_component_arr(settings.colors());
        }
        let new_config_model = slint::ModelRc::new(slint::VecModel::from(configs));
        match chip.as_str() {
            "LR35902" => main_window.set_config_lr35902(new_config_model),
            _ => continue
        };
    }

    let mut options = Rc::new(RefCell::new(RendererOptions::default()));

    let (rt_handle, rt_tx) = {
        let main_window_weak = main_window.as_weak();
        render_thread::render_thread(move |msg| {
            match msg {
                RenderThreadMessage::Error(e) => {
                    slint::invoke_from_event_loop(move || {
                        let error_message = format!("Render thread reported error: {}\
                                                           \n\nThe program will now exit", e);
                        display_error_dialog(&error_message);
                        slint::quit_event_loop().unwrap();
                    }).unwrap();
                }
                RenderThreadMessage::RenderStarting => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(true);
                        main_window_weak.unwrap().set_progress(0.0);
                        main_window_weak.unwrap().set_progress_bar_text("Setting up renderer...".into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderProgress(p) => {
                    let current_video_size = HumanBytes(p.encoded_size as u64);
                    let current_video_duration = FormattedDuration(p.encoded_duration);
                    let expected_video_duration = match p.expected_duration {
                        Some(duration) => FormattedDuration(duration).to_string(),
                        None => "?".to_string()
                    };
                    let elapsed_duration = FormattedDuration(p.elapsed_duration);
                    let eta_duration = match p.eta_duration {
                        Some(duration) => FormattedDuration(duration).to_string(),
                        None => "?".to_string()
                    };
                    let song_position = match p.song_position {
                        Some(position) => position.to_string(),
                        None => "?".to_string()
                    };

                    let status_lines = vec![
                        format!(
                            "FPS: {}, Encoded: {}/{}, Output size: {}",
                            p.average_fps,
                            current_video_duration, expected_video_duration,
                            current_video_size
                        ),
                        format!(
                            "Elapsed/ETA: {}/{}, Driver position: {}, Loop count: {}",
                            elapsed_duration, eta_duration,
                            song_position,
                            p.loop_count
                        )
                    ];
                    let (progress, progress_bar_text) = match p.expected_duration_frames {
                        Some(exp_dur_frames) => {
                            let progress = p.frame as f64 / exp_dur_frames as f64;
                            (progress, format!("{}%", (progress * 100.0) as usize))
                        },
                        None => (0.0, "Waiting for loop detection...".to_string()),
                    };

                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_progress(progress as f32);
                        main_window_weak.unwrap().set_progress_bar_text(progress_bar_text.into());
                        main_window_weak.unwrap().set_progress_lines(slint_string_arr(status_lines));
                    }).unwrap();
                }
                RenderThreadMessage::RenderComplete => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress(1.0);
                        main_window_weak.unwrap().set_progress_bar_text("100%".into());
                        main_window_weak.unwrap().set_progress_lines(slint_string_arr(vec![
                            "Done!".to_string()
                        ]));
                    }).unwrap();
                }
                RenderThreadMessage::RenderCancelled => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress(0.0);
                        main_window_weak.unwrap().set_progress_bar_text("Idle".into());
                        main_window_weak.unwrap().set_progress_lines(slint_string_arr(vec![
                            "Render cancelled.".to_string()
                        ]));
                    }).unwrap();
                }
            }
        })
    };

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_browse_for_rom(move || {
            match browse_for_rom_dialog() {
                Some(path) => {
                    main_window_weak.unwrap().set_rom_path(path.clone().into());
                    main_window_weak.unwrap().set_sav_path("".into());
                    main_window_weak.unwrap().set_input_type(InputType::None);
                    main_window_weak.unwrap().set_input_valid(false);

                    main_window_weak.unwrap().set_selected_track_index(-1);
                    main_window_weak.unwrap().set_selected_track_text("Select a track...".into());

                    main_window_weak.unwrap().set_track_duration_num("300".into());
                    main_window_weak.unwrap().set_track_duration_type("seconds".into());
                    main_window_weak.unwrap().invoke_update_formatted_duration();

                    if let Ok(Some(lsdj_version)) = lsdj::get_lsdj_version(&path) {
                        let major = i32::from_str(lsdj_version.split(".").next().unwrap()).unwrap();
                        if major < 5 {
                            display_error_dialog("Unsupported LSDj version! Please select a ROM that is v5.x or newer.");
                            return;
                        }
                        main_window_weak.unwrap().set_sav_path("".into());
                        main_window_weak.unwrap().set_input_type(InputType::LSDj);
                        main_window_weak.unwrap().set_input_valid(false);
                        options.borrow_mut().input = RenderInput::LSDj(path.clone(), "".to_string());
                        return;
                    }

                    if let Ok(gbs) = Gbs::open(path.clone()) {
                        println!(
                            "{} - {} - {} ({} tracks, start at {})",
                            gbs.title().unwrap(), gbs.artist().unwrap(), gbs.copyright().unwrap(),
                            gbs.song_count(), gbs.starting_song()
                        );
                        let track_titles: Vec<String> = (0..gbs.song_count())
                            .map(|i| format!("Track {}", i + 1))
                            .collect();
                        main_window_weak.unwrap().set_track_titles(slint_string_arr(track_titles));

                        main_window_weak.unwrap().set_input_valid(true);
                        main_window_weak.unwrap().set_input_type(InputType::GBS);
                        options.borrow_mut().input = RenderInput::GBS(path.clone());
                        return;
                    }

                    if let Ok(vgm_s) = vgm::Vgm::open(path.clone()) {
                        let song_title = match vgm_s.gd3_metadata() {
                            Some(gd3) => gd3.title,
                            None => "<?>".to_string()
                        };
                        main_window_weak.unwrap().set_track_titles(slint_string_arr(vec![song_title]));

                        main_window_weak.unwrap().set_input_valid(true);
                        main_window_weak.unwrap().set_input_type(InputType::VGM);
                        options.borrow_mut().input = RenderInput::VGM(path.clone());
                        return;
                    }

                    display_error_dialog("Unrecognized input file.");
                    main_window_weak.unwrap().set_rom_path("".into());
                    options.borrow_mut().input = RenderInput::None;
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let mut options = options.clone();
        main_window.on_browse_for_background(move || {
            match browse_for_background_dialog() {
                Some(path) => {
                    main_window_weak.unwrap().set_background_path(path.clone().into());

                    options.borrow_mut().video_options.background_path = Some(path.into());
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_browse_for_sav(move || {
            match browse_for_sav_dialog() {
                Some(path) => {
                    main_window_weak.unwrap().set_sav_path(path.clone().into());

                    main_window_weak.unwrap().set_selected_track_index(-1);
                    main_window_weak.unwrap().set_selected_track_text("Select a track...".into());

                    main_window_weak.unwrap().set_track_duration_num("300".into());
                    main_window_weak.unwrap().set_track_duration_type("seconds".into());
                    main_window_weak.unwrap().invoke_update_formatted_duration();

                    if let Ok(Some(track_titles)) = lsdj::get_track_titles_from_save(path.clone()) {
                        main_window_weak.unwrap().set_input_type(InputType::LSDj);
                        main_window_weak.unwrap().set_input_valid(true);

                        options.borrow_mut().input = RenderInput::LSDj(main_window_weak.unwrap().get_rom_path().to_string(), path.clone());

                        main_window_weak.unwrap().set_track_titles(slint_string_arr(track_titles));
                    }
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let mut options = options.clone();
        main_window.on_update_formatted_duration(move || {
            if main_window_weak.unwrap().get_selected_track_index() == -1 {
                main_window_weak.unwrap().set_track_duration_formatted("<unknown>".into());
            }

            let new_duration_type = main_window_weak.unwrap()
                .get_track_duration_type()
                .to_string();
            let new_duration_num = main_window_weak.unwrap()
                .get_track_duration_num()
                .to_string();

            let stop_condition_str = match new_duration_type.as_str() {
                "seconds" => format!("time:{}", new_duration_num),
                "frames" => format!("frames:{}", new_duration_num),
                "loops" => format!("loops:{}", new_duration_num),
                _ => unreachable!()
            };
            if let Ok(stop_condition) = StopCondition::from_str(&stop_condition_str) {
                options.borrow_mut().stop_condition = stop_condition;

                let label = match stop_condition {
                    StopCondition::Frames(frames) => {
                        let seconds = frames as f64 / 60.0;
                        FormattedDuration(Duration::from_secs_f64(seconds)).to_string()
                    },
                    StopCondition::Loops(loops) => {
                        if let RenderInput::VGM(vgm_path) = options.borrow().input.clone() {
                            let vgm_s = vgm::Vgm::open(vgm_path).unwrap();
                            let frames = vgm::duration_frames(&vgm_s, loops);
                            let seconds = frames as f64 / 60.0;
                            FormattedDuration(Duration::from_secs_f64(seconds)).to_string()
                        } else {
                            "<unknown>".to_string()
                        }
                    }
                };
                main_window_weak.unwrap().set_track_duration_formatted(label.into());
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let mut options = options.clone();
        let rt_tx = rt_tx.clone();
        main_window.on_start_render(move || {
            if !main_window_weak.unwrap().get_input_valid() {
                display_error_dialog("Invalid input file(s).");
                return;
            }

            let output_path = match browse_for_video_dialog() {
                Some(path) => path,
                None => return
            };

            if output_path.ends_with(".mov") && confirm_prores_export_dialog() {
                // Fairly close approximation of the Game Boy's frame rate with a timebase denominator <100000.
                // Required to avoid "codec timebase is very high" warning from the QuickTime encoder.
                options.borrow_mut().video_options.video_time_base = (1_097, 65_536).into();
                // -c:v prores_ks -profile:v 4 -bits_per_mb 1000 -pix_fmt yuva444p10le
                options.borrow_mut().video_options.video_codec = "prores_ks".to_string();
                options.borrow_mut().video_options.video_codec_params.insert("profile".to_string(), "4".to_string());
                options.borrow_mut().video_options.video_codec_params.insert("bits_per_mb".to_string(), "1000".to_string());
                options.borrow_mut().video_options.pixel_format_out = "yuva444p10le".to_string();
            }

            options.borrow_mut().video_options.output_path = output_path;

            let stop_condition = options.borrow().stop_condition.clone();
            let render_input = options.borrow().input.clone();
            match stop_condition {
                StopCondition::Loops(loops) => {
                    if main_window_weak.unwrap().get_input_type() == InputType::GBS {
                        display_error_dialog("Loop detection is not supported for GBS files. Please select a different duration type.");
                        return;
                    } else if let RenderInput::VGM(vgm_path) = render_input {
                        let vgm_s = vgm::Vgm::open(vgm_path).unwrap();
                        let frames = vgm::duration_frames(&vgm_s, loops);
                        options.borrow_mut().stop_condition = StopCondition::Frames(frames as u64);
                    }
                },
                _ => ()
            };

            let track_index = match main_window_weak.unwrap().get_selected_track_index() {
                -1 => {
                    display_error_dialog("Please select a track to play.");
                    return;
                },
                index => index as u8
            };
            options.borrow_mut().track_index = track_index;

            options.borrow_mut().fadeout_length = main_window_weak.unwrap().get_fadeout_duration() as u64;
            options.borrow_mut().video_options.resolution_out.0 = main_window_weak.unwrap().get_output_width() as u32;
            options.borrow_mut().video_options.resolution_out.1 = main_window_weak.unwrap().get_output_height() as u32;

            options.borrow_mut().model = match main_window_weak.unwrap().get_selected_model_text().to_string().as_str() {
                "DMG-B" => Model::DMG(Revision::RevB),
                "CGB-0" => Model::CGB(Revision::Rev0),
                "CGB-A" => Model::CGB(Revision::RevA),
                "CGB-B" => Model::CGB(Revision::RevB),
                "CGB-C" => Model::CGB(Revision::RevC),
                "CGB-D" => Model::CGB(Revision::RevD),
                "CGB-E" => Model::CGB(Revision::RevE),
                "MGB" => Model::MGB,
                "AGB" => Model::AGB,
                _ => unreachable!()
            };

            let mut channel_settings = get_default_channel_settings();
            for ((chip, channel), settings) in channel_settings.iter_mut() {
                let configs_model = match chip.as_str() {
                    "LR35902" => main_window_weak.unwrap().get_config_lr35902(),
                    _ => continue
                };
                let config = configs_model
                    .as_any()
                    .downcast_ref::<slint::VecModel<ChannelConfig>>()
                    .unwrap()
                    .iter()
                    .find(|cfg| cfg.name.to_string() == channel.clone())
                    .unwrap();

                let colors: Vec<raqote::Color> = config.colors
                    .as_any()
                    .downcast_ref::<slint::VecModel<slint::ModelRc<i32>>>()
                    .unwrap()
                    .iter()
                    .map(|color_model| {
                        let mut component_iter = color_model
                            .as_any()
                            .downcast_ref::<slint::VecModel<i32>>()
                            .unwrap()
                            .iter();
                        let r = component_iter.next().unwrap() as u8;
                        let g = component_iter.next().unwrap() as u8;
                        let b = component_iter.next().unwrap() as u8;

                        raqote::Color::new(0xFF, r, g, b)
                    })
                    .collect();

                settings.set_hidden(config.hidden);
                settings.set_colors(&colors);
            }
            options.borrow_mut().channel_settings = channel_settings;

            if main_window_weak.unwrap().get_background_path().is_empty() {
                options.borrow_mut().video_options.background_path = None;
            }

            rt_tx.send(RenderThreadRequest::StartRender(options.borrow().clone())).unwrap();
        });
    }

    {
        let rt_tx = rt_tx.clone();
        main_window.on_cancel_render(move || {
            rt_tx.send(RenderThreadRequest::CancelRender).unwrap();
        });
    }

    main_window.run().unwrap();

    if rt_tx.send(RenderThreadRequest::Terminate).is_ok() {
        // If the send failed, the channel is closed, so the thread is probably already dead.
        rt_handle.join().unwrap();
    }
}