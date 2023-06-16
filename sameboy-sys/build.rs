extern crate cc;
extern crate bindgen;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io;
use std::io::{Read, Write};

// cat ..\..\CLionProjects\gb-presenter-rs\sameboy-sys\external\SameBoy\Core\apu.h | \
// .\cppp.exe -DGB_DISABLE_TIMEKEEPING -DGB_DISABLE_REWIND -DGB_DISABLE_DEBUGGER -DGB_DISABLE_CHEATS -UGB_INTERNAL

fn run_cppp<P: AsRef<Path>>(input_path: P, output_path: P) {
    let mut cppp = Command::new("cppp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .args([
            "-DGB_DISABLE_TIMEKEEPING",
            "-DGB_DISABLE_REWIND",
            "-DGB_DISABLE_DEBUGGER",
            "-DGB_DISABLE_CHEATS",
            "-UGB_INTERNAL"
        ])
        .spawn()
        .expect("Could not start cppp! Make sure it's on your PATH.");

    let mut input_file = io::BufReader::new(fs::File::open(input_path).expect("Could not open input file!"));
    let mut output_file = io::BufWriter::new(fs::File::create(output_path).expect("Could not open output file!"));

    let mut input_src = String::new();
    input_file.read_to_string(&mut input_src).expect("Failed to read input file");
    let input_src = input_src.replace("'", "@SINGLE_QUOTE@");

    let cppp_stdin = cppp.stdin.as_mut().unwrap();
    cppp_stdin.write_all(&input_src.into_bytes()).unwrap();
    // Drop to close stdin (send EOF)
    drop(cppp_stdin);

    let cppp_output = cppp.wait_with_output().unwrap();
    if !cppp_output.status.success() {
        panic!("cppp reported failure");
    }

    let output_src = String::from_utf8(cppp_output.stdout).unwrap();
    let output_src = output_src.replace("@SINGLE_QUOTE@", "'");
    output_file.write_all(&output_src.into_bytes()).expect("Failed to write output file");
}

#[cfg(not(target_env = "msvc"))]
fn configure_cc_build(build: &mut cc::Build) -> &mut cc::Build {
    build
}

#[cfg(target_env = "msvc")]
fn configure_cc_build(build: &mut cc::Build) -> &mut cc::Build {
    build
        .compiler("clang")
        .include("external/SameBoy/Windows")
}

fn main() {
    println!("cargo:rerun-if-changed=rs-wrapper.h");

    // Preprocess all header files
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let sameboy_include_dir = out_dir.join("sameboy-include");
    if !sameboy_include_dir.exists() {
        fs::create_dir(&sameboy_include_dir).unwrap();
    }

    for entry in fs::read_dir("external/SameBoy/Core").unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            continue;
        }

        let input_path = entry.path();

        let extension = input_path.extension();
        if extension.is_none() {
            continue;
        }

        let output_path = sameboy_include_dir.join(input_path.file_name().unwrap());
        match extension.unwrap().to_str().unwrap() {
            "h" => run_cppp(input_path, output_path),
            "c" => {
                fs::copy(input_path, output_path).unwrap();
            },
            _ => ()
        }
    }

    let sameboy_version = String::from_utf8(fs::read("external/SameBoy/version.mk").unwrap())
        .unwrap()
        .replace("VERSION := ", "");

    configure_cc_build(&mut cc::Build::new())
        .include("external/SameBoy/Core")
        .flag("-ffast-math")
        .flag("-Werror")
        .flag("-Wall")
        .flag("-Wno-unknown-warning")
        .flag("-Wno-unknown-warning-option")
        .flag("-Wno-missing-braces")
        .flag("-Wno-nonnull")
        .flag("-Wno-unused-result")
        .flag("-Wno-strict-aliasing")
        .flag("-Wno-multichar")
        .flag("-Wno-int-in-bool-context")
        .flag("-Wno-format-truncation")
        .flag("-Wno-sign-compare")
        .flag("-Wno-deprecated-declarations")
        .flag("-Wno-gnu-null-pointer-arithmetic")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-constant-conversion")
        .flag("-Wno-missing-field-initializers")
        .flag("-DGB_INTERNAL")
        .flag("-DGB_DISABLE_TIMEKEEPING")
        .flag("-DGB_DISABLE_REWIND")
        .flag("-DGB_DISABLE_DEBUGGER")
        .flag("-DGB_DISABLE_CHEATS")
        .flag(format!("-DGB_VERSION=\"{}\"", sameboy_version).as_str())
        .flag("-DGB_COPYRIGHT_YEAR=2023")
        .file("external/SameBoy/Core/apu.c")
        .file("external/SameBoy/Core/camera.c")
        .file("external/SameBoy/Core/display.c")
        .file("external/SameBoy/Core/gb.c")
        .file("external/SameBoy/Core/joypad.c")
        .file("external/SameBoy/Core/mbc.c")
        .file("external/SameBoy/Core/memory.c")
        .file("external/SameBoy/Core/printer.c")
        .file("external/SameBoy/Core/random.c")
        .file("external/SameBoy/Core/rumble.c")
        .file("external/SameBoy/Core/save_state.c")
        .file("external/SameBoy/Core/sgb.c")
        .file("external/SameBoy/Core/sm83_cpu.c")
        .file("external/SameBoy/Core/timing.c")
        .file("external/SameBoy/Core/workboy.c")
        .compile("sameboy");

    let bindings = bindgen::Builder::default()
        .clang_args([format!("-I{}", sameboy_include_dir.to_str().unwrap())].iter())
        .header("rs-wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Failed to write bindings");
}

