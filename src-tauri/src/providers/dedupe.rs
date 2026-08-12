use crate::providers::{Package, ProviderKind};
use std::collections::HashMap;

/// Cross-provider + Chocolatey family dedupe for Updates.
/// Default authority: winget > chocolatey > scoop (winget tracks real desktop apps best).
pub fn dedupe_for_updates(packages: Vec<Package>, preferred: ProviderKind) -> Vec<Package> {
    let mut best: HashMap<String, Package> = HashMap::new();

    for pkg in packages {
        let key = match_key(&pkg);
        match best.get(&key) {
            None => {
                best.insert(key, pkg);
            }
            Some(existing) => {
                if should_replace(existing, &pkg, preferred) {
                    best.insert(key, pkg);
                }
            }
        }
    }

    let mut out: Vec<Package> = best.into_values().collect();
    out.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.provider.as_str().cmp(b.provider.as_str()))
    });
    out
}

fn should_replace(existing: &Package, candidate: &Package, preferred: ProviderKind) -> bool {
    let existing_rank = provider_rank(existing.provider, preferred);
    let candidate_rank = provider_rank(candidate.provider, preferred);
    if candidate_rank != existing_rank {
        return candidate_rank < existing_rank;
    }

    // Same provider: prefer concrete Chocolatey `.install` over meta package.
    choco_family_rank(&candidate.name) < choco_family_rank(&existing.name)
}

fn provider_rank(provider: ProviderKind, preferred: ProviderKind) -> u8 {
    if provider == preferred {
        return 0;
    }
    match provider {
        ProviderKind::Winget => 1,
        ProviderKind::Chocolatey => 2,
        ProviderKind::Scoop => 3,
    }
}

fn choco_family_rank(name: &str) -> u8 {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".install") {
        0
    } else if lower.ends_with(".portable") || lower.ends_with(".app") || lower.ends_with(".commandline")
    {
        2
    } else {
        1
    }
}

fn match_key(pkg: &Package) -> String {
    let mut raw = pkg.name.to_ascii_lowercase();

    // Chocolatey package family: git / git.install / git.portable → git
    for suffix in [
        ".install",
        ".portable",
        ".app",
        ".commandline",
        ".pre",
        ".extension",
    ] {
        if let Some(stripped) = raw.strip_suffix(suffix) {
            raw = stripped.to_string();
            break;
        }
    }

    // Drop trailing architecture / version noise: "7-zip 25.00 (x64)"
    raw = strip_trailing_version_noise(&raw);

    // Compact separators
    let compact: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();

    canonicalize_alias(&compact)
}

fn strip_trailing_version_noise(name: &str) -> String {
    let mut s = name.trim().to_string();
    // Remove parenthetical tags: (x64), (x86), etc.
    while let Some(start) = s.rfind('(') {
        if s.ends_with(')') {
            s = s[..start].trim_end().to_string();
        } else {
            break;
        }
    }
    // Remove trailing dotted version tokens: "7-zip 25.00"
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() >= 2 {
        let last = parts[parts.len() - 1];
        if looks_like_version(last) {
            return parts[..parts.len() - 1].join(" ");
        }
    }
    s
}

fn looks_like_version(s: &str) -> bool {
    let mut has_digit = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            has_digit = true;
        } else if c != '.' && c != '-' && c != '_' {
            return false;
        }
    }
    has_digit
}

fn canonicalize_alias(compact: &str) -> String {
    match compact {
        // Browsers
        "googlechrome" | "chrome" | "chromium" => "chrome".into(),
        "mozillafirefox" | "firefox" => "firefox".into(),
        "microsoftedge" | "edge" => "edge".into(),
        // Dev tools
        "git" => "git".into(),
        "githubcli" | "gh" => "gh".into(),
        "nodejs" | "node" => "nodejs".into(),
        "python" | "python3" => "python".into(),
        "visualstudiocode" | "vscode" | "code" => "vscode".into(),
        // Compression / media
        "7zip" | "7zipzstd" | "7zipalpha" => "7zip".into(),
        "vlc" | "vlcmediaplayer" => "vlc".into(),
        "itunes" | "appleitunes" => "itunes".into(),
        "powertoys" | "microsoftpowertoys" => "powertoys".into(),
        "notepadplusplus" | "npp" => "notepadplusplus".into(),
        "microsoftteams" | "teams" => "teams".into(),
        "zoom" | "zoomworkplace" | "zoomus" => "zoom".into(),
        "discord" => "discord".into(),
        "spotify" => "spotify".into(),
        "steam" => "steam".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(provider: ProviderKind, name: &str) -> Package {
        Package {
            id: format!("{}:{name}", provider.as_str()),
            provider,
            name: name.to_string(),
            version: Some("1".into()),
            available_version: Some("2".into()),
            summary: None,
            category: None,
            source: None,
            pinned: false,
            outdated: true,
        }
    }

    #[test]
    fn prefers_winget_over_chocolatey() {
        let out = dedupe_for_updates(
            vec![
                pkg(ProviderKind::Chocolatey, "GoogleChrome"),
                pkg(ProviderKind::Winget, "Google Chrome"),
            ],
            ProviderKind::Winget,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].provider, ProviderKind::Winget);
    }

    #[test]
    fn collapses_choco_install_family() {
        let out = dedupe_for_updates(
            vec![
                pkg(ProviderKind::Chocolatey, "git"),
                pkg(ProviderKind::Chocolatey, "git.install"),
            ],
            ProviderKind::Winget,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "git.install");
    }

    #[test]
    fn matches_7zip_variants() {
        let out = dedupe_for_updates(
            vec![
                pkg(ProviderKind::Winget, "7-Zip 25.00 (x64)"),
                pkg(ProviderKind::Chocolatey, "7zip"),
                pkg(ProviderKind::Chocolatey, "7zip.install"),
            ],
            ProviderKind::Winget,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].provider, ProviderKind::Winget);
    }
}
