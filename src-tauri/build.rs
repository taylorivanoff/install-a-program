use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    if env::var("CARGO_FEATURE_EMBEDDED_RUNNER").is_err() || env::var("IAP_STAGING_RUNNER").is_ok() {
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

    let built = manifest_dir
        .join("target")
        .join(&profile)
        .join("install-a-program-runner.exe");

    eprintln!("Building install-a-program-runner for embedded standalone export…");
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&manifest_dir)
        .env("IAP_STAGING_RUNNER", "1")
        .args([
            "build",
            "--bin",
            "install-a-program-runner",
            "--no-default-features",
            "--profile",
            &profile,
        ])
        .status()
        .expect("failed to spawn cargo for runner build");

    if !status.success() {
        panic!("failed to build install-a-program-runner for embedded template");
    }

    if !built.is_file() {
        panic!(
            "install-a-program-runner.exe missing after build; run `npm run build:runner` first"
        );
    }

    println!("cargo:rerun-if-changed=src/bin/runner-standalone.rs");
    fs::copy(&built, &dest).expect("failed to copy built runner to OUT_DIR");
}
