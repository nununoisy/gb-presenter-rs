#[cfg(windows)]
extern crate winres;

use slint_build;

fn compile(path: &str) {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent-dark".to_string());
    slint_build::compile_with_config(path, config).unwrap();
}

#[cfg(windows)]
fn apply_windows_resources() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("gb-presenter-icon.ico");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn apply_windows_resources() {
}

fn ffmpeg_sys_version_detect() {
    for (name, _value) in std::env::vars() {
        if name.starts_with("DEP_FFMPEG_") {
            println!(
                r#"cargo:rustc-cfg=feature="{}""#,
                name["DEP_FFMPEG_".len()..name.len()].to_lowercase()
            );
        }
    }
}

fn main() {
    ffmpeg_sys_version_detect();
    apply_windows_resources();
    compile("src/gui/slint/color-picker.slint");
    compile("src/gui/slint/channel-config.slint");
    compile("src/gui/slint/main.slint");
}
