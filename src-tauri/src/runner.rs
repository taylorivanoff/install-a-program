use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageProgress {
    pub id: String,
    pub display_name: String,
    pub status: String,
    pub message: Option<String>,
    pub line: Option<String>,
    pub exit_code: Option<i32>,
}

pub fn emit_progress(
    app: &AppHandle,
    id: &str,
    display_name: &str,
    status: &str,
    message: Option<String>,
    line: Option<String>,
    exit_code: Option<i32>,
) {
    let _ = app.emit(
        "package-progress",
        PackageProgress {
            id: id.to_string(),
            display_name: display_name.to_string(),
            status: status.to_string(),
            message,
            line,
            exit_code,
        },
    );
}

pub fn run_capturing(exe: &Path, args: &[&str]) -> Result<(i32, String), String> {
    let output = Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to start {}: {e}", exe.display()))?;

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

pub fn run_streaming(
    app: &AppHandle,
    id: &str,
    display_name: &str,
    exe: &Path,
    args: &[&str],
) -> Result<i32, String> {
    let mut child = Command::new(exe)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to start {}: {e}", exe.display()))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let app_out = app.clone();
    let id_out = id.to_string();
    let name_out = display_name.to_string();
    let out_handle = thread::spawn(move || {
        if let Some(out) = stdout {
            let reader = BufReader::new(out);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim_end().to_string();
                if !trimmed.is_empty() {
                    emit_progress(
                        &app_out,
                        &id_out,
                        &name_out,
                        "running",
                        None,
                        Some(trimmed),
                        None,
                    );
                }
            }
        }
    });

    let app_err = app.clone();
    let id_err = id.to_string();
    let name_err = display_name.to_string();
    let err_handle = thread::spawn(move || {
        if let Some(err) = stderr {
            let reader = BufReader::new(err);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim_end().to_string();
                if !trimmed.is_empty() {
                    emit_progress(
                        &app_err,
                        &id_err,
                        &name_err,
                        "running",
                        None,
                        Some(trimmed),
                        None,
                    );
                }
            }
        }
    });

    let status = child
        .wait()
        .map_err(|e| format!("Failed waiting for {}: {e}", exe.display()))?;
    let _ = out_handle.join();
    let _ = err_handle.join();
    Ok(status.code().unwrap_or(-1))
}

/// Run a command without piping stdio (winget downloads can fail with InternetOpenUrl
/// when stdout/stderr are redirected). Stream progress by polling a log file instead.
pub fn run_streaming_via_log(
    app: &AppHandle,
    id: &str,
    display_name: &str,
    exe: &Path,
    args: &[&str],
) -> Result<(i32, String), String> {
    let log_path: PathBuf = std::env::temp_dir().join(format!(
        "install-a-program-{}-{}.log",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&log_path);

    let mut cmdline = quote_cmd_arg(&exe.display().to_string());
    for arg in args {
        cmdline.push(' ');
        cmdline.push_str(&quote_cmd_arg(arg));
    }
    cmdline.push_str(" >");
    cmdline.push_str(&quote_cmd_arg(&log_path.display().to_string()));
    cmdline.push_str(" 2>&1");

    let mut child = Command::new("cmd.exe")
        .args(["/D", "/C", &cmdline])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to start {}: {e}", exe.display()))?;

    let mut offset = 0u64;
    let mut collected = String::new();
    loop {
        offset = tail_log(app, id, display_name, &log_path, offset, &mut collected);

        match child.try_wait() {
            Ok(Some(status)) => {
                // Final flush — winget may still be writing briefly.
                thread::sleep(Duration::from_millis(150));
                let _ = tail_log(app, id, display_name, &log_path, offset, &mut collected);
                let _ = std::fs::remove_file(&log_path);
                return Ok((status.code().unwrap_or(-1), collected));
            }
            Ok(None) => thread::sleep(Duration::from_millis(200)),
            Err(err) => {
                let _ = std::fs::remove_file(&log_path);
                return Err(format!("Failed waiting for {}: {err}", exe.display()));
            }
        }
    }
}

pub fn is_winget_network_error(code: i32, log: &str) -> bool {
    // 0x80072F78 ERROR_INTERNET_INVALID_SERVER_RESPONSE / InternetOpenUrl failures
    if code == -2147012744 || code as u32 == 0x8007_2F78 {
        return true;
    }
    let lower = log.to_ascii_lowercase();
    lower.contains("internetopenurl")
        || lower.contains("0x80072f78")
        || lower.contains("0x80072ee2")
        || lower.contains("0x80072efd")
}

fn tail_log(
    app: &AppHandle,
    id: &str,
    display_name: &str,
    log_path: &Path,
    offset: u64,
    collected: &mut String,
) -> u64 {
    let Ok(mut file) = File::open(log_path) else {
        return offset;
    };
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return offset;
    }
    let mut buf = String::new();
    if file.read_to_string(&mut buf).is_err() || buf.is_empty() {
        return offset;
    }
    let new_offset = offset + buf.len() as u64;
    for line in buf.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        if !collected.is_empty() {
            collected.push('\n');
        }
        collected.push_str(trimmed);
        emit_progress(
            app,
            id,
            display_name,
            "running",
            None,
            Some(trimmed.to_string()),
            None,
        );
    }
    new_offset
}

fn quote_cmd_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".into();
    }
    if !arg
        .chars()
        .any(|c| matches!(c, ' ' | '\t' | '"' | '&' | '|' | '<' | '>' | '^' | '%'))
    {
        return arg.to_string();
    }
    let escaped = arg.replace('"', "\"\"");
    format!("\"{escaped}\"")
}
