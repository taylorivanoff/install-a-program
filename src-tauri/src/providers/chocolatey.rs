use crate::bootstrap::find_choco;
use crate::providers::{package_id, ChocoSource, Package, ProviderKind};
use crate::runner::{emit_progress, run_capturing, run_streaming};
use tauri::AppHandle;

pub fn list_installed() -> Result<Vec<Package>, String> {
    let choco = find_choco().ok_or_else(|| "Chocolatey is not installed".to_string())?;
    let (code, text) = run_capturing(&choco, &["list", "--local-only", "--limit-output"])?;
    if code != 0 {
        return Err(format_cli_error("choco list", code, &text));
    }
    Ok(parse_limit_output(&text, false))
}

pub fn search(query: &str) -> Result<Vec<Package>, String> {
    let choco = find_choco().ok_or_else(|| "Chocolatey is not installed".to_string())?;
    let (code, text) = run_capturing(
        &choco,
        &["search", query, "--limit-output", "--page-size=50"],
    )?;
    if code != 0 {
        return Err(format_cli_error("choco search", code, &text));
    }
    Ok(parse_limit_output(&text, false))
}

pub fn list_outdated() -> Result<Vec<Package>, String> {
    let choco = find_choco().ok_or_else(|| "Chocolatey is not installed".to_string())?;
    let (code, text) = run_capturing(&choco, &["outdated", "--limit-output"])?;
    // choco outdated returns non-zero when packages are outdated on some versions
    if code != 0 && text.trim().is_empty() {
        return Err(format_cli_error("choco outdated", code, &text));
    }
    Ok(parse_outdated(&text))
}

pub fn list_sources() -> Result<Vec<ChocoSource>, String> {
    let choco = find_choco().ok_or_else(|| "Chocolatey is not installed".to_string())?;
    let (code, text) = run_capturing(&choco, &["source", "list", "--limit-output"])?;
    if code != 0 {
        return Err(format_cli_error("choco source list", code, &text));
    }
    Ok(parse_sources(&text))
}

pub fn add_source(name: &str, url: &str) -> Result<(), String> {
    let choco = find_choco().ok_or_else(|| "Chocolatey is not installed".to_string())?;
    let (code, text) = run_capturing(
        &choco,
        &["source", "add", "--name", name, "--source", url, "--force"],
    )?;
    if code != 0 {
        return Err(format_cli_error("choco source add", code, &text));
    }
    Ok(())
}

pub fn remove_source(name: &str) -> Result<(), String> {
    let choco = find_choco().ok_or_else(|| "Chocolatey is not installed".to_string())?;
    let (code, text) = run_capturing(&choco, &["source", "remove", "--name", name])?;
    if code != 0 {
        return Err(format_cli_error("choco source remove", code, &text));
    }
    Ok(())
}

pub fn run_action(
    app: &AppHandle,
    package_name: &str,
    display_name: &str,
    action: &str,
) -> Result<(), String> {
    let choco = find_choco().ok_or_else(|| "Chocolatey is not installed".to_string())?;
    let id = package_id(ProviderKind::Chocolatey, package_name);

    emit_progress(
        app,
        &id,
        display_name,
        "running",
        Some(format!("Chocolatey {action}…")),
        None,
        None,
    );

    let args: Vec<&str> = match action {
        "install" => vec!["install", package_name, "-y", "--no-progress"],
        "uninstall" => vec!["uninstall", package_name, "-y", "--no-progress"],
        "upgrade" => vec!["upgrade", package_name, "-y", "--no-progress"],
        "pin" => vec!["pin", "add", "--name", package_name],
        "unpin" => vec!["pin", "remove", "--name", package_name],
        _ => return Err(format!("Unsupported Chocolatey action: {action}")),
    };

    let code = run_streaming(app, &id, display_name, &choco, &args)?;
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
            Some(format!("choco exited with code {code}")),
            None,
            Some(code),
        );
        Err(format!("choco {action} failed (exit {code})"))
    }
}

fn parse_limit_output(text: &str, outdated: bool) -> Vec<Package> {
    let mut packages = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Chocolatey") {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].trim();
        if name.is_empty() || name.eq_ignore_ascii_case("packages matching") {
            continue;
        }
        let version = non_empty(parts[1].trim());
        packages.push(Package {
            id: package_id(ProviderKind::Chocolatey, name),
            provider: ProviderKind::Chocolatey,
            name: name.to_string(),
            version: version.clone(),
            available_version: None,
            summary: parts.get(2).and_then(|s| non_empty(s.trim())),
            category: None,
            source: None,
            pinned: false,
            outdated,
        });
    }
    packages
}

fn parse_outdated(text: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Chocolatey") {
            continue;
        }
        // id|current|available|pinned
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0].trim();
        if name.is_empty() {
            continue;
        }
        let pinned = parts
            .get(3)
            .map(|p| p.trim().eq_ignore_ascii_case("true") || p.trim() == "1")
            .unwrap_or(false);
        packages.push(Package {
            id: package_id(ProviderKind::Chocolatey, name),
            provider: ProviderKind::Chocolatey,
            name: name.to_string(),
            version: non_empty(parts[1].trim()),
            available_version: non_empty(parts[2].trim()),
            summary: None,
            category: None,
            source: None,
            pinned,
            outdated: true,
        });
    }
    packages
}

fn parse_sources(text: &str) -> Vec<ChocoSource> {
    let mut sources = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Name|Url|Disabled|User|Pass|Priority|BypassProxy|SelfService|AdminOnly
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].trim();
        if name.is_empty() {
            continue;
        }
        let disabled = parts
            .get(2)
            .map(|p| {
                let t = p.trim();
                t.eq_ignore_ascii_case("true") || t == "1"
            })
            .unwrap_or(false);
        let priority = parts.get(5).and_then(|p| p.trim().parse().ok());
        sources.push(ChocoSource {
            name: name.to_string(),
            url: parts[1].trim().to_string(),
            disabled,
            priority,
        });
    }
    sources
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
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
