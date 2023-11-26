#[cfg(windows)]
extern crate winres;

use std::path::Path;
use slint_build;

// FFmpeg vcpkg line:
// .\vcpkg\vcpkg.exe install ffmpeg[core,ffmpeg,swresample,swscale,avdevice,x264]:x64-windows --recurse

fn compile(path: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = slint_build::CompilerConfiguration::new()
        .with_include_paths(vec![
            manifest_dir.join("assets")
        ])
        .with_style("fluent-dark".to_string());
    slint_build::compile_with_config(path, config).unwrap();
}

#[cfg(windows)]
fn apply_windows_resources() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/gb-presenter-icon.ico");
    res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
    <asmv3:application>
        <asmv3:windowsSettings xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">
            <dpiAwareness>PerMonitorV2, PerMonitor, System, unaware</dpiAwareness>
        </asmv3:windowsSettings>
    </asmv3:application>
</assembly>
    "#);
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
