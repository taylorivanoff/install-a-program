use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    stage_runner_template();
    build_tauri();
}

fn build_tauri() {
    let mut windows = tauri_build::WindowsAttributes::new();
    // Release builds require admin (machine-wide uninstalls). Debug uses asInvoker
    // so `tauri dev` and cargo tests do not force a UAC prompt every launch.
    let manifest = if env::var("PROFILE").as_deref() == Ok("release") {
        include_str!("windows/app.manifest")
    } else {
        include_str!("windows/app.debug.manifest")
    };
    windows = windows.app_manifest(manifest);
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri build script");
}

fn stage_runner_template() {
    // Skip when the embedded-runner feature is off (e.g. `npm run build:runner`
    // uses --no-default-features). Never spawn a nested `cargo build` of this
    // same package — that deadlocks on Cargo's target lock (CI hung for hours).
    if env::var("CARGO_FEATURE_EMBEDDED_RUNNER").is_err() {
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest = Path::new(&out_dir).join("runner-template.exe");

    let candidates = [
        manifest_dir.join("resources/runner-template.exe"),
        manifest_dir
            .join("target")
            .join(&profile)
            .join("install-a-program-runner.exe"),
        manifest_dir
            .join("target/release/install-a-program-runner.exe"),
        manifest_dir
            .join("target/debug/install-a-program-runner.exe"),
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            println!("cargo:rerun-if-changed={}", candidate.display());
            fs::copy(candidate, &dest).expect("failed to copy runner template to OUT_DIR");
            return;
        }
    }

    panic!(
        "Standalone runner template missing. Run `npm run build:runner` (or copy \
         install-a-program-runner.exe to src-tauri/resources/runner-template.exe) \
         before building with the embedded-runner feature."
    );
}
