use slint_build;

fn compile(path: &str) {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent-dark".to_string());
    slint_build::compile_with_config(path, config).unwrap();
}

fn main() {
    compile("src/gui/slint/main.slint");
}
