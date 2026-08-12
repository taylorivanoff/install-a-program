use crate::providers::Package;

/// Fill missing categories from known package ids / name heuristics.
pub fn enrich_categories(packages: &mut [Package]) {
    for pkg in packages.iter_mut() {
        if pkg.category.as_ref().is_some_and(|c| !c.trim().is_empty()) {
            continue;
        }
        pkg.category = Some(infer_category(pkg).to_string());
    }
}

pub fn infer_category(pkg: &Package) -> &'static str {
    let id = pkg.id.to_ascii_lowercase();
    let name = pkg.name.to_ascii_lowercase();
    let bare = id
        .split_once(':')
        .map(|(_, rest)| rest)
        .unwrap_or(id.as_str());
    let compact: String = format!("{bare}{name}")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();

    if matches_any(
        &compact,
        &[
            "chrome",
            "firefox",
            "edge",
            "brave",
            "opera",
            "torbrowser",
            "vivaldi",
            "waterfox",
        ],
    ) {
        return "Browsers";
    }
    if matches_any(
        &compact,
        &[
            "7zip", "winrar", "peazip", "everything", "treesize", "bandizip",
        ],
    ) {
        return "Compression";
    }
    if matches_any(
        &compact,
        &[
            "vlc",
            "spotify",
            "itunes",
            "audacity",
            "obsstudio",
            "handbrake",
            "mpchc",
            "foobar2000",
            "kodi",
            "plex",
            "aimp",
            "potplayer",
        ],
    ) {
        return "Media";
    }
    if matches_any(
        &compact,
        &[
            "paintdotnet",
            "paintnet",
            "gimp",
            "inkscape",
            "sharex",
            "lightshot",
            "greenshot",
            "irfanview",
            "xnview",
            "blender",
        ],
    ) {
        return "Imaging";
    }
    if matches_any(
        &compact,
        &[
            "acrobat",
            "adobereader",
            "sumatrapdf",
            "libreoffice",
            "openoffice",
            "onlyoffice",
            "foxit",
            "cutepdf",
        ],
    ) {
        return "Documents";
    }
    if matches_any(
        &compact,
        &[
            "discord",
            "slack",
            "zoom",
            "teams",
            "telegram",
            "whatsapp",
            "signal",
            "skype",
            "thunderbird",
            "mailspring",
        ],
    ) {
        return "Communication";
    }
    if matches_any(
        &compact,
        &[
            "steam",
            "epicgames",
            "goggalaxy",
            "origin",
            "eaapp",
            "ubisoft",
            "battle",
            "minecraft",
        ],
    ) {
        return "Gaming";
    }
    if matches_any(
        &compact,
        &[
            "git",
            "github",
            "vscode",
            "visualstudiocode",
            "notepadplusplus",
            "sublime",
            "jetbrains",
            "nodejs",
            "python",
            "rustup",
            "golang",
            "docker",
            "postman",
            "insomnia",
            "windowsterminal",
            "powershell",
            "pwsh",
            "powertoys",
            "wsl",
            "ohmyposh",
            "winscp",
            "putty",
            "filezilla",
            "wireshark",
            "sysinternals",
            "cmake",
            "ninja",
            "yarn",
            "pnpm",
            "neovim",
            "vim",
            "emacs",
            "androidstudio",
            "intellij",
            "pycharm",
            "webstorm",
        ],
    ) {
        return "Development";
    }
    if matches_any(
        &compact,
        &[
            "malwarebytes",
            "bitwarden",
            "1password",
            "keepass",
            "lastpass",
            "nordvpn",
            "expressvpn",
            "authy",
            "verity",
            "clamav",
        ],
    ) {
        return "Security";
    }
    if matches_any(
        &compact,
        &[
            "dropbox",
            "googledrive",
            "onedrive",
            "notion",
            "obsidian",
            "onenote",
            "evernote",
            "megasync",
            "nextcloud",
        ],
    ) {
        return "Cloud";
    }
    if matches_any(
        &compact,
        &[
            "dotnet",
            "temurin",
            "openjdk",
            "jre",
            "jdk",
            "vcredist",
            "directx",
        ],
    ) {
        return "Runtimes";
    }
    if matches_any(
        &compact,
        &[
            "cpu-z",
            "cpuz",
            "hwmonitor",
            "crystaldisk",
            "qbittorrent",
            "etcher",
            "rufus",
            "ccleaner",
            "revo",
            "speccy",
            "hwinfo",
        ],
    ) {
        return "Utilities";
    }

    "Other"
}

fn matches_any(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}
