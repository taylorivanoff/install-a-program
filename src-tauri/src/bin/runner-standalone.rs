use std::env;
use std::process;
use std::thread;
use std::time::Duration;

use install_a_program_lib::bootstrap::find_winget;
use install_a_program_lib::runner::run_capturing;
use install_a_program_lib::standalone::{extract_bundle_from_bytes, extract_bundle_from_path};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--extract-bundle") {
        let exe = env::current_exe().map_err(|e| format!("Could not resolve exe path: {e}"))?;
        let bundle = extract_bundle_from_path(&exe)?;
        println!("{}", bundle.to_json()?);
        return Ok(());
    }

    let exe = env::current_exe().map_err(|e| format!("Could not resolve exe path: {e}"))?;
    let bytes = std::fs::read(&exe).map_err(|e| format!("Failed to read exe: {e}"))?;
    let bundle = extract_bundle_from_bytes(&bytes)?;

    if let Some(name) = &bundle.name {
        println!("Install Many Programs — {name}");
    } else {
        println!("Install Many Programs — standalone installer");
    }
    println!("Installing {} package(s) via winget…", bundle.ids.len());

    let winget = find_winget().ok_or_else(|| {
        "winget is not installed. Install App Installer from the Microsoft Store, then run this installer again.".to_string()
    })?;

    let mut failed = 0usize;
    for id in &bundle.ids {
        let package_id = id
            .strip_prefix("winget:")
            .ok_or_else(|| format!("Unsupported package id: {id}"))?;
        println!();
        println!("→ {package_id}");
        let args = [
            "install",
            "--id",
            package_id,
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
            "-h",
        ];
        let (code, output) = run_capturing(&winget, &args)?;
        if !output.trim().is_empty() {
            print!("{output}");
        }
        if code != 0 {
            eprintln!("Failed to install {package_id} (exit {code})");
            failed += 1;
        } else {
            println!("Installed {package_id}");
        }
        thread::sleep(Duration::from_millis(250));
    }

    println!();
    if failed > 0 {
        return Err(format!(
            "Finished with {failed} failure(s) out of {} package(s).",
            bundle.ids.len()
        ));
    }
    println!("All {} package(s) installed successfully.", bundle.ids.len());
    Ok(())
}
