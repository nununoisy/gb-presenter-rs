mod render_thread;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;
use indicatif::{FormattedDuration, HumanBytes, HumanDuration};
use native_dialog::{FileDialog, MessageDialog, MessageType};
use slint;
use slint::{Color, Model as _};
use sameboy::{Model, Revision};
use crate::config::Config;
use crate::gui::render_thread::{RenderThreadMessage, RenderThreadRequest};
use crate::renderer::gbs::Gbs;
use crate::renderer::{lsdj, m3u_searcher, vgm};
use crate::renderer::render_options::{RendererOptions, RenderInput, StopCondition};

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

fn slint_color_component_arr<I: IntoIterator<Item = tiny_skia::Color>>(a: I) -> slint::ModelRc<slint::ModelRc<i32>> {
    let color_vecs: Vec<slint::ModelRc<i32>> = a.into_iter()
        .map(|c| c.to_color_u8())
        .map(|c| slint::ModelRc::new(slint::VecModel::from(vec![
            c.red() as i32, c.green() as i32, c.blue() as i32
        ])))
        .collect();
    slint::ModelRc::new(slint::VecModel::from(color_vecs))
}

fn browse_for_rom_dialog(for_2x: bool) -> Option<String> {
    let file = if !for_2x {
        FileDialog::new()
            .add_filter("All supported formats", &["gb", "gbs", "vgm"])
            .add_filter("LSDj ROMs", &["gb"])
            .add_filter("GameBoy Sound Files", &["gbs"])
            .add_filter("Furnace/DefleMask VGMs", &["vgm"])
    } else {
        FileDialog::new()
            .add_filter("LSDj ROMs", &["gb"])
    }.show_open_single_file();

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

fn browse_for_config_import_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("Configuration File", &["toml"])
        .show_open_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
}

fn browse_for_config_export_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("Configuration File", &["toml"])
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

    main_window.set_version(env!("CARGO_PKG_VERSION").into());
    main_window.set_sameboy_version(sameboy::SAMEBOY_VERSION.into());
    main_window.set_ffmpeg_version(crate::video_builder::ffmpeg_version().into());

    let options = Rc::new(RefCell::new(RendererOptions::default()));
    let track_durations: Rc<RefCell<HashMap<u8, Duration>>> = Rc::new(RefCell::new(HashMap::new()));

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_update_channel_configs(move |write_to_config| {
            let config = &mut options.borrow_mut().config;
            let mut channel_settings = config.piano_roll.settings.to_map();
            for ((chip, channel), settings) in channel_settings.iter_mut() {
                let configs_model = match chip.as_str() {
                    "LR35902" => main_window_weak.unwrap().get_config_lr35902(),
                    "LR35902 (2x)" => main_window_weak.unwrap().get_config_lr35902_2x(),
                    _ => continue
                };
                let mut configs: Vec<ChannelConfig> = configs_model
                    .as_any()
                    .downcast_ref::<slint::VecModel<ChannelConfig>>()
                    .unwrap()
                    .iter()
                    .collect();
                let config = configs.iter_mut()
                    .find(|cfg| cfg.name.to_string() == channel.clone())
                    .unwrap();

                if !write_to_config {
                    config.hidden = settings.hidden();
                    config.colors = slint_color_component_arr(settings.colors());
                    // Hack to force Slint to recreate the ChannelConfigRow components
                    // since the Switch component sometimes ignores the model update.
                    // It can be removed when Slint adds 2-way bindings to struct elements.
                    for configs in [Vec::new(), configs] {
                        let new_config_model = slint::ModelRc::new(slint::VecModel::from(configs));
                        match chip.as_str() {
                            "LR35902" => main_window_weak.unwrap().set_config_lr35902(new_config_model),
                            "LR35902 (2x)" => main_window_weak.unwrap().set_config_lr35902_2x(new_config_model),
                            _ => continue
                        };
                    }
                } else {
                    let colors: Vec<tiny_skia::Color> = config.colors
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

                            tiny_skia::Color::from_rgba8(r, g, b, 0xFF)
                        })
                        .collect();

                    settings.set_hidden(config.hidden);
                    settings.set_colors(&colors);
                }
            }

            if write_to_config {
                config.piano_roll.settings.apply_from_map(&channel_settings);
            }
            main_window_weak.unwrap().window().request_redraw();
        });
    }
    main_window.invoke_update_channel_configs(false);

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_import_config(move || {
            match browse_for_config_import_dialog() {
                Some(path) => {
                    let new_config_str = match fs::read_to_string(path) {
                        Ok(d) => d,
                        Err(e) => return display_error_dialog(&e.to_string()),
                    };
                    options.borrow_mut().config = match Config::from_toml(&new_config_str) {
                        Ok(c) => c,
                        Err(e) => return display_error_dialog(&e.to_string())
                    };
                    main_window_weak.unwrap().invoke_update_channel_configs(false);
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_export_config(move || {
            match browse_for_config_export_dialog() {
                Some(path) => {
                    main_window_weak.unwrap().invoke_update_channel_configs(true);

                    let config_str = match options.borrow().config.export() {
                        Ok(c) => c,
                        Err(e) => return display_error_dialog(&e.to_string())
                    };

                    match fs::write(&path, config_str) {
                        Ok(()) => (),
                        Err(e) => return display_error_dialog(&e.to_string())
                    }
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_reset_config(move || {
            options.borrow_mut().config = Config::default();
            main_window_weak.unwrap().invoke_update_channel_configs(false);
        });
    }

    let (rt_handle, rt_tx) = {
        let main_window_weak = main_window.as_weak();
        render_thread::render_thread(move |msg| {
            match msg {
                RenderThreadMessage::Error(e) => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_indeterminate(false);
                        main_window_weak.unwrap().set_progress_error(true);
                        main_window_weak.unwrap().set_progress_title("Idle".into());
                        main_window_weak.unwrap().set_progress_status(format!("Render error: {}", e).into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderStarting => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(true);
                        main_window_weak.unwrap().set_progress_indeterminate(true);
                        main_window_weak.unwrap().set_progress_error(false);
                        main_window_weak.unwrap().set_progress(0.0);
                        main_window_weak.unwrap().set_progress_title("Setting up".into());
                        main_window_weak.unwrap().set_progress_status("Preparing your song".into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderProgress(p) => {
                    let current_video_size = HumanBytes(p.encoded_size as u64);
                    let current_video_duration = FormattedDuration(p.encoded_duration);
                    let expected_video_duration = match p.expected_duration {
                        Some(duration) => FormattedDuration(duration).to_string(),
                        None => "(unknown)".to_string()
                    };
                    // let elapsed_duration = FormattedDuration(p.elapsed_duration);
                    let eta_duration = match p.eta_duration {
                        Some(duration) => HumanDuration(duration.saturating_sub(p.elapsed_duration)).to_string(),
                        None => "Unknown time".to_string()
                    };

                    let (progress, progress_title) = match (p.frame, p.expected_duration_frames) {
                        (frame, Some(exp_dur_frames)) => {
                            let progress = frame as f64 / exp_dur_frames as f64;
                            (progress, "Rendering".to_string())
                        },
                        (0, None) => (0.0, "Initializing".to_string()),
                        (_, None) => (0.0, "Rendering to loop point".to_string()),
                    };
                    let progress_status = format!(
                        "{}%, {} FPS, encoded {}/{} ({}), {} remaining",
                        (progress * 100.0).round(),
                        p.average_fps,
                        current_video_duration, expected_video_duration,
                        current_video_size,
                        eta_duration
                    );

                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_progress_indeterminate(p.expected_duration_frames.is_none());
                        main_window_weak.unwrap().set_progress(progress as f32);
                        main_window_weak.unwrap().set_progress_title(progress_title.into());
                        main_window_weak.unwrap().set_progress_status(progress_status.into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderComplete => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_indeterminate(false);
                        main_window_weak.unwrap().set_progress(1.0);
                        main_window_weak.unwrap().set_progress_title("Idle".into());
                        main_window_weak.unwrap().set_progress_status("Finished".into());
                    }).unwrap();
                }
                RenderThreadMessage::RenderCancelled => {
                    let main_window_weak = main_window_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        main_window_weak.unwrap().set_rendering(false);
                        main_window_weak.unwrap().set_progress_indeterminate(false);
                        main_window_weak.unwrap().set_progress_title("Idle".into());
                        main_window_weak.unwrap().set_progress_status("Render cancelled".into());
                    }).unwrap();
                }
            }
        })
    };

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        let track_durations = track_durations.clone();
        main_window.on_browse_for_rom(move |for_2x| {
            match browse_for_rom_dialog(for_2x) {
                Some(path) => {
                    track_durations.borrow_mut().clear();

                    main_window_weak.unwrap().set_rom_path(path.clone().into());
                    main_window_weak.unwrap().set_sav_path("".into());
                    main_window_weak.unwrap().set_input_type(InputType::None);
                    main_window_weak.unwrap().set_input_valid(false);

                    if !for_2x {
                        main_window_weak.unwrap().set_track_titles(slint::ModelRc::new(slint::VecModel::from(Vec::new())));
                        main_window_weak.unwrap().set_selected_track_index(-1);
                        main_window_weak.unwrap().set_selected_track_text("Select a track...".into());
                    } else {
                        debug_assert!(main_window_weak.unwrap().invoke_is_2x(), "Tried to set 2x SAV in non-2x mode");

                        main_window_weak.unwrap().set_track_titles_2x(slint::ModelRc::new(slint::VecModel::from(Vec::new())));
                        main_window_weak.unwrap().set_selected_track_index_2x(-1);
                        main_window_weak.unwrap().set_selected_track_text_2x("Select a track...".into());
                    }

                    main_window_weak.unwrap().set_track_duration_num("300".into());
                    main_window_weak.unwrap().set_track_duration_type("seconds".into());
                    main_window_weak.unwrap().invoke_update_formatted_duration();

                    let lsdj_version = lsdj::get_lsdj_version(&path);
                    if let Ok(lsdj_version) = lsdj_version {
                        let major = i32::from_str(lsdj_version.split(".").next().unwrap_or_default()).unwrap_or(0);
                        if major < 3 {
                            display_error_dialog("Unsupported LSDj version! Please select a ROM that is v3.x or newer.");
                            return;
                        }
                        main_window_weak.unwrap().set_sav_path("".into());
                        main_window_weak.unwrap().set_input_type(InputType::LSDj);
                        main_window_weak.unwrap().set_input_valid(false);
                        if !for_2x {
                            if !main_window_weak.unwrap().invoke_is_2x() {
                                options.borrow_mut().input = RenderInput::LSDj(path.clone(), "".to_string());
                            } else {
                                options.borrow_mut().input = RenderInput::LSDj2x(
                                    path.clone(),
                                    "".to_string(),
                                    main_window_weak.unwrap().get_rom_path_2x().to_string(),
                                    main_window_weak.unwrap().get_sav_path_2x().to_string()
                                );
                            }
                        } else {
                            options.borrow_mut().input = RenderInput::LSDj2x(
                                main_window_weak.unwrap().get_rom_path().to_string(),
                                main_window_weak.unwrap().get_sav_path().to_string(),
                                path.clone(),
                                "".to_string()
                            );
                        }
                        return;
                    }

                    if for_2x {
                        display_error_dialog(format!("Error opening 2x LSDj ROM!\n{}", lsdj_version.err().unwrap()).as_str());
                        return;
                    }

                    let gbs = Gbs::open(path.clone());
                    if let Ok(gbs) = gbs {
                        println!(
                            "{} - {} - {} ({} tracks, start at {})",
                            gbs.title().unwrap(), gbs.artist().unwrap(), gbs.copyright().unwrap(),
                            gbs.song_count(), gbs.starting_song()
                        );
                        let m3u_titles = match m3u_searcher::search(path.clone()) {
                            Ok(t) => t,
                            Err(e) => {
                                println!("M3U search failed: {}", e);
                                HashMap::default()
                            }
                        };
                        let track_titles: Vec<String> = (0..gbs.song_count())
                            .map(|i| {
                                match m3u_titles.get(&i) {
                                    Some((title, duration)) => {
                                        if duration.is_some() {
                                            track_durations.borrow_mut().insert(i, duration.unwrap().clone());
                                        }
                                        format!("Track {}: {}", i + 1, title.clone())
                                    },
                                    None => format!("Track {}", i + 1)
                                }
                            })
                            .collect();
                        main_window_weak.unwrap().set_track_titles(slint_string_arr(track_titles));

                        main_window_weak.unwrap().set_input_valid(true);
                        main_window_weak.unwrap().set_input_type(InputType::GBS);
                        options.borrow_mut().input = RenderInput::GBS(path.clone());
                        return;
                    }

                    let vgm_s = vgm::Vgm::open(path.clone());
                    if let Ok(vgm_s) = vgm_s {
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

                    display_error_dialog(format!(
                        "Unrecognized input file!\n\nWhile opening as LSDj ROM: {}\nWhile opening as GBS: {}\nWhile opening as VGM: {}",
                        lsdj_version.err().unwrap(),
                        gbs.err().unwrap(),
                        vgm_s.err().unwrap()
                    ).as_str());
                    main_window_weak.unwrap().set_rom_path("".into());
                    options.borrow_mut().input = RenderInput::None;
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
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
        main_window.on_browse_for_sav(move |for_2x| {
            match browse_for_sav_dialog() {
                Some(path) => {
                    main_window_weak.unwrap().set_sav_path(path.clone().into());

                    if !for_2x {
                        main_window_weak.unwrap().set_track_titles(slint::ModelRc::new(slint::VecModel::from(Vec::new())));
                        main_window_weak.unwrap().set_selected_track_index(-1);
                        main_window_weak.unwrap().set_selected_track_text("Select a track...".into());
                    } else {
                        debug_assert!(main_window_weak.unwrap().invoke_is_2x(), "Tried to set 2x SAV in non-2x mode");

                        main_window_weak.unwrap().set_track_titles_2x(slint::ModelRc::new(slint::VecModel::from(Vec::new())));
                        main_window_weak.unwrap().set_selected_track_index_2x(-1);
                        main_window_weak.unwrap().set_selected_track_text_2x("Select a track...".into());
                    }

                    main_window_weak.unwrap().set_track_duration_num("300".into());
                    main_window_weak.unwrap().set_track_duration_type("seconds".into());
                    main_window_weak.unwrap().invoke_update_formatted_duration();

                    match lsdj::get_track_titles_from_save(path.clone()) {
                        Ok(track_titles) => {
                            main_window_weak.unwrap().set_input_type(InputType::LSDj);
                            main_window_weak.unwrap().set_input_valid(true);

                            if !for_2x {
                                if !main_window_weak.unwrap().invoke_is_2x() {
                                    options.borrow_mut().input = RenderInput::LSDj(
                                        main_window_weak.unwrap().get_rom_path().to_string(),
                                        path.clone()
                                    );
                                } else {
                                    options.borrow_mut().input = RenderInput::LSDj2x(
                                        main_window_weak.unwrap().get_rom_path().to_string(),
                                        path.clone(),
                                        main_window_weak.unwrap().get_rom_path_2x().to_string(),
                                        main_window_weak.unwrap().get_sav_path_2x().to_string(),
                                    );
                                }

                                main_window_weak.unwrap().set_track_titles(slint_string_arr(track_titles));
                            } else {
                                options.borrow_mut().input = RenderInput::LSDj2x(
                                    main_window_weak.unwrap().get_rom_path().to_string(),
                                    main_window_weak.unwrap().get_sav_path().to_string(),
                                    main_window_weak.unwrap().get_rom_path_2x().to_string(),
                                    path.clone(),
                                );

                                main_window_weak.unwrap().set_track_titles_2x(slint_string_arr(track_titles));
                            }
                        }
                        Err(e) => {
                            main_window_weak.unwrap().set_input_type(InputType::LSDj);
                            main_window_weak.unwrap().set_input_valid(false);

                            display_error_dialog(&e.to_string());
                        }
                    }
                },
                None => ()
            }
        });
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
        main_window.on_set_lsdj_2x(move |is_2x| {
            match main_window_weak.unwrap().get_input_type() {
                InputType::LSDj => (),
                _ => return false
            }

            let rom_path = main_window_weak.unwrap().get_rom_path().to_string();
            let sav_path = main_window_weak.unwrap().get_sav_path().to_string();

            if is_2x {
                options.borrow_mut().input = RenderInput::LSDj2x(
                    rom_path.clone(),
                    sav_path.clone(),
                    rom_path.clone(),
                    sav_path.clone()
                );
                main_window_weak.unwrap().set_rom_path_2x(rom_path.into());
                main_window_weak.unwrap().set_sav_path_2x(sav_path.into());

                main_window_weak.unwrap().set_track_titles_2x(main_window_weak.unwrap().get_track_titles().clone());
            } else {
                options.borrow_mut().input = RenderInput::LSDj(
                    rom_path.clone(),
                    sav_path.clone()
                );
                main_window_weak.unwrap().set_rom_path_2x("".into());
                main_window_weak.unwrap().set_sav_path_2x("".into());

                main_window_weak.unwrap().set_track_titles_2x(slint::ModelRc::new(slint::VecModel::from(Vec::new())));
            }

            is_2x
        })
    }

    {
        let main_window_weak = main_window.as_weak();
        let options = options.clone();
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
        let options = options.clone();
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

            if output_path.ends_with(".mov") {
                // Fairly close approximation of the Game Boy's frame rate with a timebase denominator <100000.
                // Required to avoid "codec timebase is very high" warning from the QuickTime encoder.
                options.borrow_mut().video_options.video_time_base = (1_097, 65_536).into();

                if confirm_prores_export_dialog() {
                    // -c:v prores_ks -profile:v 4 -bits_per_mb 1000 -pix_fmt yuva444p10le
                    options.borrow_mut().video_options.video_codec = "prores_ks".to_string();
                    options.borrow_mut().video_options.video_codec_params.insert("profile".to_string(), "4".to_string());
                    options.borrow_mut().video_options.video_codec_params.insert("bits_per_mb".to_string(), "1000".to_string());
                    options.borrow_mut().video_options.pixel_format_out = "yuva444p10le".to_string();
                }
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

            if main_window_weak.unwrap().invoke_is_2x() {
                let track_index_2x = match main_window_weak.unwrap().get_selected_track_index_2x() {
                    -1 => {
                        display_error_dialog("Please select a track to play on the 2x Game Boy.");
                        return;
                    },
                    index => index as u8
                };
                options.borrow_mut().track_index_2x = track_index_2x;
            }

            options.borrow_mut().fadeout_length = main_window_weak.unwrap().get_fadeout_duration() as u64;
            options.borrow_mut().set_resolution_smart(
                main_window_weak.unwrap().get_output_width() as u32,
                main_window_weak.unwrap().get_output_height() as u32
            );

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

            main_window_weak.unwrap().invoke_update_channel_configs(true);

            if main_window_weak.unwrap().get_background_path().is_empty() {
                options.borrow_mut().video_options.background_path = None;
            }

            options.borrow_mut().auto_lsdj_sync = true;

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