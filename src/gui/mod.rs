mod render_thread;

use std::cell::RefCell;
use std::path;
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;
use indicatif::{FormattedDuration, HumanBytes};
use native_dialog::{FileDialog, MessageDialog, MessageType};
use slint;
use crate::gui::render_thread::RenderThreadMessage;
use crate::main;
use crate::renderer::gbs::Gbs;
use crate::renderer::lsdj;
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

fn browse_for_rom_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["gb", "gbs"])
        .add_filter("LSDj ROMs", &["gb"])
        .add_filter("GameBoy Sound Files", &["gbs"])
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

fn browse_for_video_dialog() -> Option<String> {
    let file = FileDialog::new()
        .add_filter("All supported formats", &["mp4", "mkv"])
        .add_filter("MPEG-4 Video", &["mp4"])
        .add_filter("Matroska Video", &["mkv"])
        .show_save_single_file();

    match file {
        Ok(Some(path)) => Some(path.to_str().unwrap().to_string()),
        _ => None
    }
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
                        main_window_weak.unwrap().set_rendering(true)
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
                    main_window_weak.unwrap().set_lsdj_mode(false);
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
                        main_window_weak.unwrap().set_lsdj_mode(true);
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
                        options.borrow_mut().input = RenderInput::GBS(path.clone());
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
                        main_window_weak.unwrap().set_lsdj_mode(true);
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
                    StopCondition::Loops(_) => "<unknown>".to_string()
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
            options.borrow_mut().video_options.output_path = output_path;

            match &options.borrow().stop_condition {
                StopCondition::Loops(_) => {
                    if !main_window_weak.unwrap().get_lsdj_mode() {
                        display_error_dialog("Loop detection is not supported for GBS files. Please select a different duration type.");
                        return;
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

            rt_tx.send(Some(options.borrow().clone())).unwrap();
        });
    }

    main_window.run().unwrap();

    if rt_tx.send(None).is_ok() {
        // If the send failed, the channel is closed, so the thread is probably already dead.
        rt_handle.join().unwrap();
    }
}