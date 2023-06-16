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

fn main() {
    apply_windows_resources();
    compile("src/gui/slint/main.slint");
}
