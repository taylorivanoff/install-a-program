use crate::bootstrap::find_winget;
use crate::providers::{package_id, Package, ProviderKind};
use crate::runner::{
    emit_progress, is_winget_network_error, run_capturing, run_streaming_via_log,
};
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// winget: APPINSTALLER_CLI_ERROR_NO_APPLICATIONS_FOUND
const WINGET_NO_APPS_FOUND: i32 = -1978335212; // 0x8A150014

pub fn list_installed() -> Result<Vec<Package>, String> {
    let winget = find_winget().ok_or_else(|| "winget is not installed".to_string())?;
    let (code, text) = run_capturing(
        &winget,
        &[
            "list",
            "--accept-source-agreements",
            "--disable-interactivity",
        ],
    )?;
    if code != 0 && text.trim().is_empty() {
        return Err(format_cli_error("winget list", code, &text));
    }
    Ok(parse_table(&text, false))
}

pub fn search(query: &str) -> Result<Vec<Package>, String> {
    let winget = find_winget().ok_or_else(|| "winget is not installed".to_string())?;
    let (code, text) = run_capturing(
        &winget,
        &[
            "search",
            query,
            "--accept-source-agreements",
            "--disable-interactivity",
        ],
    )?;
    if code != 0 && text.trim().is_empty() {
        return Err(format_cli_error("winget search", code, &text));
    }
    Ok(parse_table(&text, false))
}

pub fn list_outdated() -> Result<Vec<Package>, String> {
    let winget = find_winget().ok_or_else(|| "winget is not installed".to_string())?;
    let (code, text) = run_capturing(
        &winget,
        &[
            "upgrade",
            "--include-unknown",
            "--accept-source-agreements",
            "--disable-interactivity",
        ],
    )?;
    // `winget upgrade` with no package lists outdated; exit code may be non-zero.
    if text.trim().is_empty() && code != 0 {
        return Err(format_cli_error("winget upgrade (list)", code, &text));
    }
    Ok(parse_table(&text, true))
}

pub fn run_action(
    app: &AppHandle,
    package_id_name: &str,
    display_name: &str,
    action: &str,
) -> Result<(), String> {
    let winget = find_winget().ok_or_else(|| "winget is not installed".to_string())?;
    let id = package_id(ProviderKind::Winget, package_id_name);

    emit_progress(
        app,
        &id,
        display_name,
        "running",
        Some(format!("winget {action}…")),
        None,
        None,
    );

    if matches!(action, "pin" | "unpin") {
        emit_progress(
            app,
            &id,
            display_name,
            "failed",
            Some("Pin/unpin is not supported for winget in this app yet".into()),
            None,
            None,
        );
        return Err("winget pin/unpin is not supported yet".into());
    }

    if matches!(action, "install" | "upgrade") {
        close_running_apps(app, &id, display_name, package_id_name, display_name);
    }

    let exact_args = action_args(action, package_id_name, true)?;
    let (mut code, mut log) =
        run_streaming_via_log(app, &id, display_name, &winget, &exact_args)?;

    // App still running (installer exit 26 / winget message) — force-close and retry once.
    if code != 0 && is_app_in_use(&log) {
        emit_progress(
            app,
            &id,
            display_name,
            "running",
            Some("App is running — closing it and retrying…".into()),
            None,
            None,
        );
        close_running_apps(app, &id, display_name, package_id_name, display_name);
        thread::sleep(Duration::from_secs(2));
        let retry = run_streaming_via_log(app, &id, display_name, &winget, &exact_args)?;
        code = retry.0;
        log = retry.1;
    }

    // Exact id match failed — retry without -e, then by name.
    if code != 0 && is_no_apps_found(code, &log) && matches!(action, "upgrade" | "uninstall") {
        emit_progress(
            app,
            &id,
            display_name,
            "running",
            Some("No exact match — retrying without -e…".into()),
            None,
            None,
        );
        let loose_args = action_args(action, package_id_name, false)?;
        let retry = run_streaming_via_log(app, &id, display_name, &winget, &loose_args)?;
        code = retry.0;
        log = retry.1;
    }
    if code != 0 && is_no_apps_found(code, &log) && matches!(action, "upgrade" | "uninstall") {
        emit_progress(
            app,
            &id,
            display_name,
            "running",
            Some(format!("Retrying winget {action} by name “{display_name}”…")),
            None,
            None,
        );
        let name_args = action_args_by_name(action, display_name)?;
        let retry = run_streaming_via_log(app, &id, display_name, &winget, &name_args)?;
        code = retry.0;
        log = retry.1;
    }

    if code != 0 && is_winget_network_error(code, &log) {
        emit_progress(
            app,
            &id,
            display_name,
            "running",
            Some("winget download failed (network). Retrying once…".into()),
            None,
            None,
        );
        thread::sleep(Duration::from_secs(2));
        let retry = run_streaming_via_log(app, &id, display_name, &winget, &exact_args)?;
        code = retry.0;
        log = retry.1;
    }

    if code == 0 {
        emit_progress(
            app,
            &id,
            display_name,
            "done",
            Some(format!("{action} succeeded")),
            None,
            Some(code),
        );
        Ok(())
    } else {
        let hint = failure_hint(code, &log);
        emit_progress(
            app,
            &id,
            display_name,
            "failed",
            Some(format!("winget exited with code {code}.{hint}")),
            None,
            Some(code),
        );
        Err(format!("winget {action} failed (exit {code})"))
    }
}

fn action_args<'a>(action: &'a str, package_id_name: &'a str, exact: bool) -> Result<Vec<&'a str>, String> {
    let mut args = match action {
        "install" => vec![
            "install",
            "--id",
            package_id_name,
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
            "-h",
        ],
        "uninstall" => vec![
            "uninstall",
            "--id",
            package_id_name,
            "--disable-interactivity",
            "-h",
        ],
        "upgrade" => vec![
            "upgrade",
            "--id",
            package_id_name,
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
            "-h",
        ],
        "pin" | "unpin" => return Err("winget pin/unpin is not supported yet".into()),
        _ => return Err(format!("Unsupported winget action: {action}")),
    };
    if exact {
        // Insert -e after the id value (index of package id is 2)
        args.insert(3, "-e");
    }
    Ok(args)
}

fn action_args_by_name<'a>(action: &'a str, name: &'a str) -> Result<Vec<&'a str>, String> {
    match action {
        "upgrade" => Ok(vec![
            "upgrade",
            "--name",
            name,
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
            "-h",
        ]),
        "uninstall" => Ok(vec![
            "uninstall",
            "--name",
            name,
            "--disable-interactivity",
            "-h",
        ]),
        _ => Err(format!("Unsupported winget name action: {action}")),
    }
}

fn close_running_apps(
    app: &AppHandle,
    id: &str,
    display_name: &str,
    package_id_name: &str,
    name: &str,
) {
    let processes = processes_for_package(package_id_name, name);
    if processes.is_empty() {
        return;
    }
    for proc_name in processes {
        emit_progress(
            app,
            id,
            display_name,
            "running",
            Some(format!("Closing {proc_name} if running…")),
            None,
            None,
        );
        let _ = Command::new("taskkill.exe")
            .args(["/IM", proc_name, "/F", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    thread::sleep(Duration::from_millis(800));
}

fn processes_for_package(package_id: &str, display_name: &str) -> Vec<&'static str> {
    let key = format!(
        "{}{}",
        package_id.to_ascii_lowercase(),
        display_name.to_ascii_lowercase()
    );
    let compact: String = key.chars().filter(|c| c.is_ascii_alphanumeric()).collect();

    if compact.contains("spotify") {
        return vec!["Spotify.exe", "SpotifyWebHelper.exe"];
    }
    if compact.contains("discord") {
        return vec!["Discord.exe", "Update.exe"];
    }
    if compact.contains("slack") {
        return vec!["slack.exe"];
    }
    if compact.contains("teams") {
        return vec!["ms-teams.exe", "Teams.exe"];
    }
    if compact.contains("zoom") {
        return vec!["Zoom.exe"];
    }
    if compact.contains("steam") {
        return vec!["steam.exe", "steamwebhelper.exe"];
    }
    if compact.contains("chrome") && !compact.contains("chromedriver") {
        return vec!["chrome.exe"];
    }
    if compact.contains("firefox") {
        return vec!["firefox.exe"];
    }
    if compact.contains("code") || compact.contains("vscode") {
        return vec!["Code.exe"];
    }
    if compact.contains("obsidian") {
        return vec!["Obsidian.exe"];
    }
    if compact.contains("notion") {
        return vec!["Notion.exe"];
    }
    if compact.contains("telegram") {
        return vec!["Telegram.exe"];
    }
    if compact.contains("vlc") {
        return vec!["vlc.exe"];
    }
    if compact.contains("itunes") {
        return vec!["iTunes.exe"];
    }
    if compact.contains("sumatrapdf") || compact.contains("sumatra") {
        return vec!["SumatraPDF.exe"];
    }
    if compact.contains("notepadplusplus") || compact.contains("notepad++") {
        return vec!["notepad++.exe"];
    }
    Vec::new()
}

fn is_app_in_use(log: &str) -> bool {
    let lower = log.to_ascii_lowercase();
    lower.contains("application is currently running")
        || lower.contains("exit the application then try again")
        || lower.contains("installer failed with exit code: 26")
        || lower.contains("exit code: 26")
}

fn is_no_apps_found(code: i32, log: &str) -> bool {
    if code == WINGET_NO_APPS_FOUND || code as u32 == 0x8A15_0014 {
        return true;
    }
    let lower = log.to_ascii_lowercase();
    lower.contains("no installed package found")
        || lower.contains("no package found matching")
        || lower.contains("no applications matched")
}

fn failure_hint(code: i32, log: &str) -> String {
    if is_app_in_use(log) {
        return " Close the app (it was still running) and try again.".into();
    }
    if is_no_apps_found(code, log) {
        return " winget could not match that installed package (id/name mismatch). Refresh Updates or update it from Installed.".into();
    }
    if is_winget_network_error(code, log) {
        return " Network/TLS/proxy issue while downloading. Try again, check VPN/proxy, or update App Installer.".into();
    }
    if let Some(line) = log.lines().rev().find(|l| {
        let t = l.trim();
        !t.is_empty()
            && !t.to_ascii_lowercase().starts_with("installer log is available")
            && t.len() < 180
    }) {
        return format!(" {line}");
    }
    String::new()
}

fn parse_table(text: &str, force_outdated: bool) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut header_seen = false;
    let mut name_end = 0usize;
    let mut id_end = 0usize;
    let mut ver_end = 0usize;

    for line in text.lines() {
        let raw = line.trim_end();
        if raw.is_empty() {
            continue;
        }
        let lower = raw.to_ascii_lowercase();
        if lower.contains("no installed package")
            || lower.contains("no available upgrade")
            || lower.starts_with("the `msstore`")
            || lower.contains("failed when searching")
        {
            continue;
        }

        if !header_seen {
            if let Some(name_idx) = find_column(raw, "Name") {
                if let Some(id_idx) = find_column(raw, "Id") {
                    header_seen = true;
                    name_end = id_idx;
                    let ver_idx = find_column(raw, "Version").unwrap_or(raw.len());
                    id_end = ver_idx;
                    let avail_idx = find_column(raw, "Available")
                        .or_else(|| find_column(raw, "Source"))
                        .unwrap_or(raw.len());
                    ver_end = avail_idx;
                    let _ = name_idx;
                    continue;
                }
            }
            continue;
        }

        if raw.chars().all(|c| c == '-' || c == ' ') {
            continue;
        }

        let name = slice_col(raw, 0, name_end);
        let pkg_id = slice_col(raw, name_end, id_end);
        let version = slice_col(raw, id_end, ver_end);
        if name.is_empty() || pkg_id.is_empty() {
            continue;
        }

        let rest = if raw.len() > ver_end {
            raw[ver_end..].trim()
        } else {
            ""
        };
        let (available, source, outdated) = if force_outdated {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let available = parts.first().map(|s| s.to_string());
            let source = parts.get(1).map(|s| s.to_string());
            (available, source, true)
        } else {
            let source = rest.split_whitespace().next().map(|s| s.to_string());
            (None, source, false)
        };

        packages.push(Package {
            id: package_id(ProviderKind::Winget, &pkg_id),
            provider: ProviderKind::Winget,
            name,
            version: non_empty(&version),
            available_version: available,
            summary: None,
            category: None,
            source,
            pinned: false,
            outdated,
        });
    }

    packages
}

fn find_column(header: &str, name: &str) -> Option<usize> {
    header.find(name)
}

fn slice_col(line: &str, start: usize, end: usize) -> String {
    let bytes = line.as_bytes();
    if start >= bytes.len() {
        return String::new();
    }
    let end = end.min(bytes.len());
    // Prefer char-safe slicing via char indices approximation for ASCII tables
    let s = &line[start.min(line.len())..end.min(line.len())];
    s.trim().to_string()
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        None
    } else {
        Some(t.to_string())
    }
}

fn format_cli_error(cmd: &str, code: i32, text: &str) -> String {
    let detail = text.trim();
    if detail.is_empty() {
        format!("{cmd} failed (exit {code})")
    } else {
        format!("{cmd} failed (exit {code}): {detail}")
    }
}
