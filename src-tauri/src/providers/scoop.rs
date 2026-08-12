use crate::bootstrap::find_scoop;
use crate::providers::{package_id, Package, ProviderKind};
use crate::runner::{emit_progress, run_capturing, run_streaming};
use tauri::AppHandle;

pub fn list_installed() -> Result<Vec<Package>, String> {
    let scoop = find_scoop().ok_or_else(|| "Scoop is not installed".to_string())?;
    let (code, text) = run_capturing(&scoop, &["list"])?;
    if code != 0 && text.trim().is_empty() {
        return Err(format_cli_error("scoop list", code, &text));
    }
    Ok(parse_list(&text))
}

pub fn search(query: &str) -> Result<Vec<Package>, String> {
    let scoop = find_scoop().ok_or_else(|| "Scoop is not installed".to_string())?;
    let (code, text) = run_capturing(&scoop, &["search", query])?;
    if code != 0 && text.trim().is_empty() {
        return Err(format_cli_error("scoop search", code, &text));
    }
    Ok(parse_search(&text))
}

pub fn list_outdated() -> Result<Vec<Package>, String> {
    let scoop = find_scoop().ok_or_else(|| "Scoop is not installed".to_string())?;
    let (code, text) = run_capturing(&scoop, &["status"])?;
    if code != 0 && text.trim().is_empty() {
        return Err(format_cli_error("scoop status", code, &text));
    }
    Ok(parse_status_outdated(&text))
}

pub fn run_action(
    app: &AppHandle,
    package_name: &str,
    display_name: &str,
    action: &str,
) -> Result<(), String> {
    let scoop = find_scoop().ok_or_else(|| "Scoop is not installed".to_string())?;
    let id = package_id(ProviderKind::Scoop, package_name);

    emit_progress(
        app,
        &id,
        display_name,
        "running",
        Some(format!("Scoop {action}…")),
        None,
        None,
    );

    let args: Vec<&str> = match action {
        "install" => vec!["install", package_name],
        "uninstall" => vec!["uninstall", package_name],
        "upgrade" => vec!["update", package_name],
        "pin" | "unpin" => {
            emit_progress(
                app,
                &id,
                display_name,
                "failed",
                Some("Pin/unpin is not supported for Scoop in this app yet".into()),
                None,
                None,
            );
            return Err("Scoop pin/unpin is not supported yet".into());
        }
        _ => return Err(format!("Unsupported Scoop action: {action}")),
    };

    let code = run_streaming(app, &id, display_name, &scoop, &args)?;
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
        emit_progress(
            app,
            &id,
            display_name,
            "failed",
            Some(format!("scoop exited with code {code}")),
            None,
            Some(code),
        );
        Err(format!("scoop {action} failed (exit {code})"))
    }
}

fn parse_list(text: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut started = false;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("installed apps") || lower.starts_with("name ") {
            started = true;
            continue;
        }
        if !started {
            // scoop list often prints a header then Name Version Source Updated
            if lower.contains("name") && lower.contains("version") {
                started = true;
            }
            continue;
        }
        if line.chars().all(|c| c == '-' || c == ' ') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0];
        let version = parts[1];
        let source = parts.get(2).map(|s| s.to_string());
        packages.push(Package {
            id: package_id(ProviderKind::Scoop, name),
            provider: ProviderKind::Scoop,
            name: name.to_string(),
            version: Some(version.to_string()),
            available_version: None,
            summary: None,
            category: None,
            source,
            pinned: false,
            outdated: false,
        });
    }
    packages
}

fn parse_search(text: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut bucket = String::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if line.ends_with("bucket:") || (line.ends_with(':') && !line.contains(' ')) {
            bucket = line.trim_end_matches(':').trim().to_string();
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("name ") || lower.contains("results from") {
            continue;
        }
        if line.chars().all(|c| c == '-' || c == ' ') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let name = parts[0];
        let version = parts.get(1).map(|s| s.to_string());
        packages.push(Package {
            id: package_id(ProviderKind::Scoop, name),
            provider: ProviderKind::Scoop,
            name: name.to_string(),
            version,
            available_version: None,
            summary: None,
            category: None,
            source: if bucket.is_empty() {
                None
            } else {
                Some(bucket.clone())
            },
            pinned: false,
            outdated: false,
        });
    }
    packages
}

fn parse_status_outdated(text: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.contains("up to date") || lower.starts_with("scoop") || lower.contains("updates are available") {
            continue;
        }
        // Typical: name: version -> available
        if let Some((name, rest)) = line.split_once(':') {
            let name = name.trim();
            if name.is_empty() || name.contains(' ') {
                continue;
            }
            let rest = rest.trim();
            let (version, available) = if let Some((cur, avail)) = rest.split_once("->") {
                (Some(cur.trim().to_string()), Some(avail.trim().to_string()))
            } else {
                (Some(rest.to_string()), None)
            };
            packages.push(Package {
                id: package_id(ProviderKind::Scoop, name),
                provider: ProviderKind::Scoop,
                name: name.to_string(),
                version,
                available_version: available,
                summary: None,
                category: None,
                source: None,
                pinned: false,
                outdated: true,
            });
        }
    }
    packages
}

fn format_cli_error(cmd: &str, code: i32, text: &str) -> String {
    let detail = text.trim();
    if detail.is_empty() {
        format!("{cmd} failed (exit {code})")
    } else {
        format!("{cmd} failed (exit {code}): {detail}")
    }
}
