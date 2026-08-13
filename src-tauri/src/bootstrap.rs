use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub chocolatey: ToolStatus,
    pub winget: ToolStatus,
    pub scoop: ToolStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapProgress {
    pub provider: String,
    pub status: String,
    pub message: Option<String>,
    pub line: Option<String>,
}

pub fn provider_status() -> ProviderStatus {
    ProviderStatus {
        chocolatey: chocolatey_status(),
        winget: winget_status(),
        scoop: scoop_status(),
    }
}

pub fn chocolatey_status() -> ToolStatus {
    match find_choco() {
        Some(path) => {
            let version = probe_version(&path, &["--version"]);
            ToolStatus {
                available: true,
                path: Some(path.display().to_string()),
                version,
                message: None,
            }
        }
        None => ToolStatus {
            available: false,
            path: None,
            version: None,
            message: Some("Chocolatey not found.".into()),
        },
    }
}

pub fn winget_status() -> ToolStatus {
    match find_winget() {
        Some(path) => {
            let version = probe_version(&path, &["--version"]);
            ToolStatus {
                available: true,
                path: Some(path.display().to_string()),
                version,
                message: None,
            }
        }
        None => ToolStatus {
            available: false,
            path: None,
            version: None,
            message: Some("winget not found.".into()),
        },
    }
}

pub fn scoop_status() -> ToolStatus {
    match find_scoop() {
        Some(path) => {
            let version = probe_version(&path, &["--version"]);
            ToolStatus {
                available: true,
                path: Some(path.display().to_string()),
                version,
                message: None,
            }
        }
        None => ToolStatus {
            available: false,
            path: None,
            version: None,
            message: Some("Scoop not found.".into()),
        },
    }
}

pub fn find_choco() -> Option<PathBuf> {
    which("choco.exe")
        .or_else(|| which("choco"))
        .or_else(|| {
            let p = PathBuf::from(r"C:\ProgramData\chocolatey\bin\choco.exe");
            p.is_file().then_some(p)
        })
}

pub fn find_winget() -> Option<PathBuf> {
    which("winget.exe").or_else(|| which("winget")).or_else(|| {
        let local = std::env::var_os("LOCALAPPDATA")?;
        let base = PathBuf::from(local).join(r"Microsoft\WindowsApps\winget.exe");
        base.is_file().then_some(base)
    })
}

pub fn find_scoop() -> Option<PathBuf> {
    which("scoop.cmd")
        .or_else(|| which("scoop"))
        .or_else(|| {
            let home = std::env::var_os("USERPROFILE")?;
            let p = PathBuf::from(home).join(r"scoop\shims\scoop.cmd");
            p.is_file().then_some(p)
        })
}

fn emit_bootstrap(
    app: &AppHandle,
    provider: &str,
    status: &str,
    message: Option<String>,
    line: Option<String>,
) {
    let _ = app.emit(
        "bootstrap-progress",
        BootstrapProgress {
            provider: provider.to_string(),
            status: status.to_string(),
            message,
            line,
        },
    );
}

/// Install any missing providers sequentially, streaming output to the activity log.
pub fn ensure_providers(app: &AppHandle) -> Result<(), String> {
    emit_bootstrap(
        app,
        "system",
        "running",
        Some("Checking Chocolatey, winget, and Scoop…".into()),
        None,
    );

    let status = provider_status();
    let mut work = Vec::new();
    if !status.chocolatey.available {
        work.push("chocolatey");
    }
    if !status.winget.available {
        work.push("winget");
    }
    if !status.scoop.available {
        work.push("scoop");
    }

    if work.is_empty() {
        emit_bootstrap(
            app,
            "system",
            "done",
            Some("All package managers are ready.".into()),
            None,
        );
        let _ = app.emit("bootstrap-finished", ());
        return Ok(());
    }

    emit_bootstrap(
        app,
        "system",
        "running",
        Some(format!(
            "Installing missing package managers in the background: {}",
            work.join(", ")
        )),
        None,
    );

    for provider in work {
        match provider {
            "chocolatey" => {
                let _ = install_chocolatey_streaming(app);
            }
            "winget" => {
                let _ = install_winget_streaming(app);
            }
            "scoop" => {
                let _ = install_scoop_streaming(app);
            }
            _ => {}
        }
    }

    let final_status = provider_status();
    let summary = format!(
        "Bootstrap finished — Chocolatey: {}, winget: {}, Scoop: {}",
        if final_status.chocolatey.available {
            "ready"
        } else {
            "missing"
        },
        if final_status.winget.available {
            "ready"
        } else {
            "missing"
        },
        if final_status.scoop.available {
            "ready"
        } else {
            "missing"
        }
    );
    emit_bootstrap(app, "system", "done", Some(summary), None);
    let _ = app.emit("bootstrap-finished", ());
    Ok(())
}

/// Install winget only when missing (Simple mode bootstrap).
pub fn ensure_winget(app: &AppHandle) -> Result<(), String> {
    emit_bootstrap(
        app,
        "system",
        "running",
        Some("Checking winget…".into()),
        None,
    );

    if provider_status().winget.available {
        emit_bootstrap(
            app,
            "system",
            "done",
            Some("winget is ready.".into()),
            None,
        );
        let _ = app.emit("bootstrap-finished", ());
        return Ok(());
    }

    emit_bootstrap(
        app,
        "system",
        "running",
        Some("Installing winget (App Installer)…".into()),
        None,
    );
    let _ = install_winget_streaming(app);

    let ready = provider_status().winget.available;
    let message = if ready {
        "winget is ready.".into()
    } else {
        "winget is still missing. Open App Installer from Settings.".into()
    };
    emit_bootstrap(app, "system", if ready { "done" } else { "failed" }, Some(message), None);
    let _ = app.emit("bootstrap-finished", ());
    Ok(())
}

/// Official Chocolatey bootstrap (does not redistribute Chocolatey binaries).
pub fn install_chocolatey() -> Result<String, String> {
    if find_choco().is_some() {
        return Ok("Chocolatey is already installed.".into());
    }

    let script = chocolatey_install_script();
    let (code, text) = run_powershell_capturing(&script)?;
    if code != 0 {
        return Err(format_fail("Chocolatey", code, &text));
    }
    if find_choco().is_none() {
        return Err(
            "Bootstrap finished but choco.exe was not found. You may need to restart the app."
                .into(),
        );
    }
    Ok(if text.trim().is_empty() {
        "Chocolatey installed successfully.".into()
    } else {
        text
    })
}

fn install_chocolatey_streaming(app: &AppHandle) -> Result<(), String> {
    if find_choco().is_some() {
        emit_bootstrap(
            app,
            "chocolatey",
            "done",
            Some("Chocolatey is already installed.".into()),
            None,
        );
        return Ok(());
    }

    emit_bootstrap(
        app,
        "chocolatey",
        "running",
        Some("Installing Chocolatey (official bootstrap)…".into()),
        None,
    );

    let code = run_powershell_streaming(app, "chocolatey", &chocolatey_install_script())?;
    if code != 0 {
        emit_bootstrap(
            app,
            "chocolatey",
            "failed",
            Some(format!("Chocolatey bootstrap failed (exit {code})")),
            None,
        );
        return Err(format!("Chocolatey bootstrap failed (exit {code})"));
    }

    if find_choco().is_none() {
        emit_bootstrap(
            app,
            "chocolatey",
            "failed",
            Some("Bootstrap finished but choco.exe was not found. Restart the app if needed.".into()),
            None,
        );
        return Err("choco.exe not found after bootstrap".into());
    }

    emit_bootstrap(
        app,
        "chocolatey",
        "done",
        Some("Chocolatey installed successfully.".into()),
        None,
    );
    Ok(())
}

fn install_winget_streaming(app: &AppHandle) -> Result<(), String> {
    if find_winget().is_some() {
        emit_bootstrap(
            app,
            "winget",
            "done",
            Some("winget is already installed.".into()),
            None,
        );
        return Ok(());
    }

    emit_bootstrap(
        app,
        "winget",
        "running",
        Some("Installing winget (App Installer)…".into()),
        None,
    );

    let code = run_powershell_streaming(app, "winget", &winget_install_script())?;
    // Refresh PATH probe after AppX install
    if find_winget().is_some() {
        emit_bootstrap(
            app,
            "winget",
            "done",
            Some("winget installed successfully.".into()),
            None,
        );
        return Ok(());
    }

    if code != 0 {
        emit_bootstrap(
            app,
            "winget",
            "failed",
            Some(format!(
                "winget install failed (exit {code}). Try App Installer from the Microsoft Store."
            )),
            None,
        );
        return Err(format!("winget install failed (exit {code})"));
    }

    emit_bootstrap(
        app,
        "winget",
        "failed",
        Some(
            "App Installer step finished but winget.exe was not found yet. Open App Installer from Settings or restart the app."
                .into(),
        ),
        None,
    );
    Err("winget.exe not found after install".into())
}

fn install_scoop_streaming(app: &AppHandle) -> Result<(), String> {
    if find_scoop().is_some() {
        emit_bootstrap(
            app,
            "scoop",
            "done",
            Some("Scoop is already installed.".into()),
            None,
        );
        return Ok(());
    }

    // Do NOT use inline PowerShell download/PATH scripts — Windows Defender often
    // false-positives those as Trojan:Win32/ClickFix. Install via winget or Chocolatey.
    if let Some(winget) = find_winget() {
        emit_bootstrap(
            app,
            "scoop",
            "running",
            Some("Installing Scoop via winget (avoids Defender false positives)…".into()),
            None,
        );
        let args = [
            "install",
            "--id",
            "ScoopInstaller.Scoop",
            "-e",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
            "-h",
        ];
        match crate::runner::run_streaming_via_log(app, "scoop", "Scoop", &winget, &args) {
            Ok((code, _)) if code == 0 || find_scoop().is_some() => {
                if find_scoop().is_some() || code == 0 {
                    emit_bootstrap(
                        app,
                        "scoop",
                        "done",
                        Some("Scoop installed via winget.".into()),
                        None,
                    );
                    return Ok(());
                }
            }
            Ok((code, log)) => {
                emit_bootstrap(
                    app,
                    "scoop",
                    "running",
                    Some(format!(
                        "winget Scoop install exited {code}; trying Chocolatey if available…"
                    )),
                    None,
                );
                let _ = log;
            }
            Err(err) => {
                emit_bootstrap(
                    app,
                    "scoop",
                    "running",
                    Some(format!("winget Scoop install error: {err}; trying Chocolatey…")),
                    None,
                );
            }
        }
    }

    if let Some(choco) = find_choco() {
        emit_bootstrap(
            app,
            "scoop",
            "running",
            Some("Installing Scoop via Chocolatey…".into()),
            None,
        );
        let args = ["install", "scoop", "-y", "--no-progress"];
        match crate::runner::run_streaming(app, "scoop", "Scoop", &choco, &args) {
            Ok(code) if code == 0 || find_scoop().is_some() => {
                emit_bootstrap(
                    app,
                    "scoop",
                    "done",
                    Some("Scoop installed via Chocolatey.".into()),
                    None,
                );
                return Ok(());
            }
            Ok(code) => {
                emit_bootstrap(
                    app,
                    "scoop",
                    "running",
                    Some(format!("Chocolatey Scoop install exited {code}.")),
                    None,
                );
            }
            Err(err) => {
                emit_bootstrap(
                    app,
                    "scoop",
                    "running",
                    Some(format!("Chocolatey Scoop install error: {err}")),
                    None,
                );
            }
        }
    }

    emit_bootstrap(
        app,
        "scoop",
        "failed",
        Some(
            "Scoop was not installed automatically (Defender blocks scripted Scoop setups). Install from https://scoop.sh or: winget install ScoopInstaller.Scoop"
                .into(),
        ),
        None,
    );
    Err("Scoop auto-install unavailable; install manually via winget or scoop.sh".into())
}

fn chocolatey_install_script() -> String {
    r#"
$ErrorActionPreference = 'Continue'
# Host already launched with -ExecutionPolicy Bypass — skip Set-ExecutionPolicy.
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
Write-Host 'Downloading Chocolatey install script…'
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
Write-Host 'Chocolatey bootstrap script finished.'
"#
    .to_string()
}

fn winget_install_script() -> String {
    r#"
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Write-Host 'Downloading App Installer (winget) package…'
$temp = Join-Path $env:TEMP ('winget-installer-' + [guid]::NewGuid().ToString() + '.msixbundle')
try {
  Invoke-WebRequest -Uri 'https://aka.ms/getwinget' -OutFile $temp -UseBasicParsing
  Write-Host 'Installing App Installer package…'
  Add-AppxPackage -Path $temp -ErrorAction Stop
  Write-Host 'App Installer package installed.'
} catch {
  Write-Host ("Primary winget install failed: " + $_.Exception.Message)
  Write-Host 'Trying Microsoft Store App Installer deep link is not available in silent mode.'
  throw
} finally {
  if (Test-Path $temp) { Remove-Item -Force $temp -ErrorAction SilentlyContinue }
}
"#
    .to_string()
}

fn run_powershell_capturing(script: &str) -> Result<(i32, String), String> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to start PowerShell: {e}"))?;

    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr);
    if !err.trim().is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&err);
    }
    Ok((output.status.code().unwrap_or(-1), text))
}

fn run_powershell_streaming(app: &AppHandle, provider: &str, script: &str) -> Result<i32, String> {
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to start PowerShell: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_out = app.clone();
    let provider_out = provider.to_string();
    let out_handle = thread::spawn(move || {
        if let Some(out) = stdout {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let trimmed = line.trim_end().to_string();
                if !trimmed.is_empty() {
                    emit_bootstrap(&app_out, &provider_out, "running", None, Some(trimmed));
                }
            }
        }
    });

    let app_err = app.clone();
    let provider_err = provider.to_string();
    let err_handle = thread::spawn(move || {
        if let Some(err) = stderr {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                let trimmed = line.trim_end().to_string();
                if !trimmed.is_empty() {
                    emit_bootstrap(&app_err, &provider_err, "running", None, Some(trimmed));
                }
            }
        }
    });

    let status = child
        .wait()
        .map_err(|e| format!("Failed waiting for PowerShell: {e}"))?;
    let _ = out_handle.join();
    let _ = err_handle.join();
    Ok(status.code().unwrap_or(-1))
}

fn format_fail(name: &str, code: i32, text: &str) -> String {
    let detail = text.trim();
    if detail.is_empty() {
        format!("{name} bootstrap failed (exit {code})")
    } else {
        format!("{name} bootstrap failed (exit {code}): {detail}")
    }
}

fn probe_version(exe: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
