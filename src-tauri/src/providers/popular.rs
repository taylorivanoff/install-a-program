use crate::providers::{package_id, Package, ProviderKind};

/// Ninite-style starter catalog for the Browse tab (empty search).
struct PopularApp {
    name: &'static str,
    winget_id: Option<&'static str>,
    chocolatey_id: Option<&'static str>,
    scoop_id: Option<&'static str>,
}

const POPULAR: &[PopularApp] = &[
    // Browsers
    PopularApp {
        name: "Google Chrome",
        winget_id: Some("Google.Chrome"),
        chocolatey_id: Some("googlechrome"),
        scoop_id: Some("googlechrome"),
    },
    PopularApp {
        name: "Mozilla Firefox",
        winget_id: Some("Mozilla.Firefox"),
        chocolatey_id: Some("firefox"),
        scoop_id: Some("firefox"),
    },
    PopularApp {
        name: "Microsoft Edge",
        winget_id: Some("Microsoft.Edge"),
        chocolatey_id: Some("microsoft-edge"),
        scoop_id: None,
    },
    PopularApp {
        name: "Brave",
        winget_id: Some("Brave.Brave"),
        chocolatey_id: Some("brave"),
        scoop_id: Some("brave"),
    },
    PopularApp {
        name: "Opera",
        winget_id: Some("Opera.Opera"),
        chocolatey_id: Some("opera"),
        scoop_id: Some("opera"),
    },
    PopularApp {
        name: "Tor Browser",
        winget_id: Some("TorProject.TorBrowser"),
        chocolatey_id: Some("tor-browser"),
        scoop_id: Some("tor"),
    },
    // Compression / files
    PopularApp {
        name: "7-Zip",
        winget_id: Some("7zip.7zip"),
        chocolatey_id: Some("7zip"),
        scoop_id: Some("7zip"),
    },
    PopularApp {
        name: "WinRAR",
        winget_id: Some("RARLab.WinRAR"),
        chocolatey_id: Some("winrar"),
        scoop_id: None,
    },
    PopularApp {
        name: "PeaZip",
        winget_id: Some("Giorgiotani.Peazip"),
        chocolatey_id: Some("peazip"),
        scoop_id: Some("peazip"),
    },
    PopularApp {
        name: "Everything",
        winget_id: Some("voidtools.Everything"),
        chocolatey_id: Some("everything"),
        scoop_id: Some("everything"),
    },
    PopularApp {
        name: "TreeSize Free",
        winget_id: Some("JAMSoftware.TreeSize.Free"),
        chocolatey_id: Some("treesizefree"),
        scoop_id: None,
    },
    // Media
    PopularApp {
        name: "VLC media player",
        winget_id: Some("VideoLAN.VLC"),
        chocolatey_id: Some("vlc"),
        scoop_id: Some("vlc"),
    },
    PopularApp {
        name: "Spotify",
        winget_id: Some("Spotify.Spotify"),
        chocolatey_id: Some("spotify"),
        scoop_id: Some("spotify"),
    },
    PopularApp {
        name: "iTunes",
        winget_id: Some("Apple.iTunes"),
        chocolatey_id: Some("itunes"),
        scoop_id: None,
    },
    PopularApp {
        name: "Audacity",
        winget_id: Some("Audacity.Audacity"),
        chocolatey_id: Some("audacity"),
        scoop_id: Some("audacity"),
    },
    PopularApp {
        name: "OBS Studio",
        winget_id: Some("OBSProject.OBSStudio"),
        chocolatey_id: Some("obs-studio"),
        scoop_id: Some("obs-studio"),
    },
    PopularApp {
        name: "HandBrake",
        winget_id: Some("HandBrake.HandBrake"),
        chocolatey_id: Some("handbrake"),
        scoop_id: Some("handbrake"),
    },
    PopularApp {
        name: "MPC-HC",
        winget_id: Some("clsid2.mpc-hc"),
        chocolatey_id: Some("mpc-hc"),
        scoop_id: Some("mpc-hc"),
    },
    PopularApp {
        name: "foobar2000",
        winget_id: Some("PeterPawlowski.foobar2000"),
        chocolatey_id: Some("foobar2000"),
        scoop_id: Some("foobar2000"),
    },
    // Imaging / docs
    PopularApp {
        name: "Paint.NET",
        winget_id: Some("dotPDN.PaintDotNet"),
        chocolatey_id: Some("paint.net"),
        scoop_id: None,
    },
    PopularApp {
        name: "GIMP",
        winget_id: Some("GIMP.GIMP.3"),
        chocolatey_id: Some("gimp"),
        scoop_id: Some("gimp"),
    },
    PopularApp {
        name: "Inkscape",
        winget_id: Some("Inkscape.Inkscape"),
        chocolatey_id: Some("inkscape"),
        scoop_id: Some("inkscape"),
    },
    PopularApp {
        name: "ShareX",
        winget_id: Some("ShareX.ShareX"),
        chocolatey_id: Some("sharex"),
        scoop_id: Some("sharex"),
    },
    PopularApp {
        name: "Adobe Acrobat Reader",
        winget_id: Some("Adobe.Acrobat.Reader.64-bit"),
        chocolatey_id: Some("adobereader"),
        scoop_id: None,
    },
    PopularApp {
        name: "SumatraPDF",
        winget_id: Some("SumatraPDF.SumatraPDF"),
        chocolatey_id: Some("sumatrapdf"),
        scoop_id: Some("sumatrapdf"),
    },
    PopularApp {
        name: "LibreOffice",
        winget_id: Some("TheDocumentFoundation.LibreOffice"),
        chocolatey_id: Some("libreoffice-fresh"),
        scoop_id: Some("libreoffice"),
    },
    // Communication / social
    PopularApp {
        name: "Discord",
        winget_id: Some("Discord.Discord"),
        chocolatey_id: Some("discord"),
        scoop_id: Some("discord"),
    },
    PopularApp {
        name: "Slack",
        winget_id: Some("SlackTechnologies.Slack"),
        chocolatey_id: Some("slack"),
        scoop_id: Some("slack"),
    },
    PopularApp {
        name: "Zoom",
        winget_id: Some("Zoom.Zoom"),
        chocolatey_id: Some("zoom"),
        scoop_id: None,
    },
    PopularApp {
        name: "Microsoft Teams",
        winget_id: Some("Microsoft.Teams"),
        chocolatey_id: Some("microsoft-teams"),
        scoop_id: None,
    },
    PopularApp {
        name: "Telegram",
        winget_id: Some("Telegram.TelegramDesktop"),
        chocolatey_id: Some("telegram"),
        scoop_id: Some("telegram"),
    },
    PopularApp {
        name: "WhatsApp",
        winget_id: Some("WhatsApp.WhatsApp"),
        chocolatey_id: Some("whatsapp"),
        scoop_id: None,
    },
    PopularApp {
        name: "Signal",
        winget_id: Some("OpenWhisperSystems.Signal"),
        chocolatey_id: Some("signal"),
        scoop_id: Some("signal"),
    },
    // Gaming
    PopularApp {
        name: "Steam",
        winget_id: Some("Valve.Steam"),
        chocolatey_id: Some("steam"),
        scoop_id: Some("steam"),
    },
    PopularApp {
        name: "Epic Games Launcher",
        winget_id: Some("EpicGames.EpicGamesLauncher"),
        chocolatey_id: Some("epicgameslauncher"),
        scoop_id: None,
    },
    PopularApp {
        name: "GOG Galaxy",
        winget_id: Some("GOG.Galaxy"),
        chocolatey_id: Some("goggalaxy"),
        scoop_id: None,
    },
    // Dev tools
    PopularApp {
        name: "Git",
        winget_id: Some("Git.Git"),
        chocolatey_id: Some("git"),
        scoop_id: Some("git"),
    },
    PopularApp {
        name: "GitHub CLI",
        winget_id: Some("GitHub.cli"),
        chocolatey_id: Some("gh"),
        scoop_id: Some("gh"),
    },
    PopularApp {
        name: "GitHub Desktop",
        winget_id: Some("GitHub.GitHubDesktop"),
        chocolatey_id: Some("github-desktop"),
        scoop_id: None,
    },
    PopularApp {
        name: "Visual Studio Code",
        winget_id: Some("Microsoft.VisualStudioCode"),
        chocolatey_id: Some("vscode"),
        scoop_id: Some("vscode"),
    },
    PopularApp {
        name: "Notepad++",
        winget_id: Some("Notepad++.Notepad++"),
        chocolatey_id: Some("notepadplusplus"),
        scoop_id: Some("notepadplusplus"),
    },
    PopularApp {
        name: "Sublime Text",
        winget_id: Some("SublimeHQ.SublimeText.4"),
        chocolatey_id: Some("sublimetext4"),
        scoop_id: Some("sublime-text"),
    },
    PopularApp {
        name: "JetBrains Toolbox",
        winget_id: Some("JetBrains.Toolbox"),
        chocolatey_id: Some("jetbrainstoolbox"),
        scoop_id: None,
    },
    PopularApp {
        name: "Node.js LTS",
        winget_id: Some("OpenJS.NodeJS.LTS"),
        chocolatey_id: Some("nodejs-lts"),
        scoop_id: Some("nodejs-lts"),
    },
    PopularApp {
        name: "Python 3",
        winget_id: Some("Python.Python.3.12"),
        chocolatey_id: Some("python"),
        scoop_id: Some("python"),
    },
    PopularApp {
        name: "Rustup",
        winget_id: Some("Rustlang.Rustup"),
        chocolatey_id: Some("rustup.install"),
        scoop_id: Some("rustup"),
    },
    PopularApp {
        name: "Go",
        winget_id: Some("GoLang.Go"),
        chocolatey_id: Some("golang"),
        scoop_id: Some("go"),
    },
    PopularApp {
        name: "Docker Desktop",
        winget_id: Some("Docker.DockerDesktop"),
        chocolatey_id: Some("docker-desktop"),
        scoop_id: None,
    },
    PopularApp {
        name: "Postman",
        winget_id: Some("Postman.Postman"),
        chocolatey_id: Some("postman"),
        scoop_id: Some("postman"),
    },
    PopularApp {
        name: "Insomnia",
        winget_id: Some("Insomnia.Insomnia"),
        chocolatey_id: Some("insomnia-rest-api-client"),
        scoop_id: Some("insomnia"),
    },
    PopularApp {
        name: "Windows Terminal",
        winget_id: Some("Microsoft.WindowsTerminal"),
        chocolatey_id: Some("microsoft-windows-terminal"),
        scoop_id: Some("windows-terminal"),
    },
    PopularApp {
        name: "PowerShell",
        winget_id: Some("Microsoft.PowerShell"),
        chocolatey_id: Some("powershell-core"),
        scoop_id: Some("pwsh"),
    },
    PopularApp {
        name: "PowerToys",
        winget_id: Some("Microsoft.PowerToys"),
        chocolatey_id: Some("powertoys"),
        scoop_id: Some("powertoys"),
    },
    PopularApp {
        name: "Windows Subsystem for Linux",
        winget_id: Some("Microsoft.WSL"),
        chocolatey_id: Some("wsl2"),
        scoop_id: None,
    },
    PopularApp {
        name: "Oh My Posh",
        winget_id: Some("JanDeDobbeleer.OhMyPosh"),
        chocolatey_id: Some("oh-my-posh"),
        scoop_id: Some("oh-my-posh"),
    },
    PopularApp {
        name: "WinSCP",
        winget_id: Some("WinSCP.WinSCP"),
        chocolatey_id: Some("winscp"),
        scoop_id: Some("winscp"),
    },
    PopularApp {
        name: "PuTTY",
        winget_id: Some("PuTTY.PuTTY"),
        chocolatey_id: Some("putty"),
        scoop_id: Some("putty"),
    },
    PopularApp {
        name: "FileZilla",
        winget_id: Some("TimKosse.FileZilla.Client"),
        chocolatey_id: Some("filezilla"),
        scoop_id: Some("filezilla"),
    },
    PopularApp {
        name: "Wireshark",
        winget_id: Some("WiresharkFoundation.Wireshark"),
        chocolatey_id: Some("wireshark"),
        scoop_id: Some("wireshark"),
    },
    PopularApp {
        name: "Sysinternals Suite",
        winget_id: Some("Microsoft.Sysinternals.Suite"),
        chocolatey_id: Some("sysinternals"),
        scoop_id: None,
    },
    // Security / privacy
    PopularApp {
        name: "Malwarebytes",
        winget_id: Some("Malwarebytes.Malwarebytes"),
        chocolatey_id: Some("malwarebytes"),
        scoop_id: None,
    },
    PopularApp {
        name: "Bitwarden",
        winget_id: Some("Bitwarden.Bitwarden"),
        chocolatey_id: Some("bitwarden"),
        scoop_id: Some("bitwarden"),
    },
    PopularApp {
        name: "1Password",
        winget_id: Some("AgileBits.1Password"),
        chocolatey_id: Some("1password"),
        scoop_id: None,
    },
    PopularApp {
        name: "KeePassXC",
        winget_id: Some("KeePassXCTeam.KeePassXC"),
        chocolatey_id: Some("keepassxc"),
        scoop_id: Some("keepassxc"),
    },
    // Cloud / sync / productivity
    PopularApp {
        name: "Dropbox",
        winget_id: Some("Dropbox.Dropbox"),
        chocolatey_id: Some("dropbox"),
        scoop_id: None,
    },
    PopularApp {
        name: "Google Drive",
        winget_id: Some("Google.GoogleDrive"),
        chocolatey_id: Some("google-drive-file-stream"),
        scoop_id: None,
    },
    PopularApp {
        name: "Notion",
        winget_id: Some("Notion.Notion"),
        chocolatey_id: Some("notion"),
        scoop_id: None,
    },
    PopularApp {
        name: "Obsidian",
        winget_id: Some("Obsidian.Obsidian"),
        chocolatey_id: Some("obsidian"),
        scoop_id: Some("obsidian"),
    },
    PopularApp {
        name: "Microsoft OneNote",
        winget_id: Some("Microsoft.OneNote"),
        chocolatey_id: Some("onenote"),
        scoop_id: None,
    },
    // Runtimes / utilities
    PopularApp {
        name: ".NET Desktop Runtime 8",
        winget_id: Some("Microsoft.DotNet.DesktopRuntime.8"),
        chocolatey_id: Some("dotnet-8.0-desktopruntime"),
        scoop_id: None,
    },
    PopularApp {
        name: "Java Runtime (Temurin 17)",
        winget_id: Some("EclipseAdoptium.Temurin.17.JRE"),
        chocolatey_id: Some("temurin17jre"),
        scoop_id: Some("temurin17-jre"),
    },
    PopularApp {
        name: "CPU-Z",
        winget_id: Some("CPUID.CPU-Z"),
        chocolatey_id: Some("cpu-z"),
        scoop_id: Some("cpu-z"),
    },
    PopularApp {
        name: "HWMonitor",
        winget_id: Some("CPUID.HWMonitor"),
        chocolatey_id: Some("hwmonitor"),
        scoop_id: Some("hwmonitor"),
    },
    PopularApp {
        name: "CrystalDiskInfo",
        winget_id: Some("CrystalDewWorld.CrystalDiskInfo"),
        chocolatey_id: Some("crystaldiskinfo"),
        scoop_id: Some("crystaldiskinfo"),
    },
    PopularApp {
        name: "CrystalDiskMark",
        winget_id: Some("CrystalDewWorld.CrystalDiskMark"),
        chocolatey_id: Some("crystaldiskmark"),
        scoop_id: Some("crystaldiskmark"),
    },
    PopularApp {
        name: "qBittorrent",
        winget_id: Some("qBittorrent.qBittorrent"),
        chocolatey_id: Some("qbittorrent"),
        scoop_id: Some("qbittorrent"),
    },
    PopularApp {
        name: "BalenaEtcher",
        winget_id: Some("Balena.Etcher"),
        chocolatey_id: Some("etcher"),
        scoop_id: Some("etcher"),
    },
    PopularApp {
        name: "Rufus",
        winget_id: Some("Rufus.Rufus"),
        chocolatey_id: Some("rufus"),
        scoop_id: Some("rufus"),
    },
    PopularApp {
        name: "Foxit PDF Reader",
        winget_id: Some("Foxit.FoxitReader"),
        chocolatey_id: Some("foxitreader"),
        scoop_id: None,
    },
    PopularApp {
        name: "IrfanView",
        winget_id: Some("IrfanSkiljan.IrfanView"),
        chocolatey_id: Some("irfanview"),
        scoop_id: Some("irfanview"),
    },
    PopularApp {
        name: "Google Earth Pro",
        winget_id: Some("Google.EarthPro"),
        chocolatey_id: Some("googleearthpro"),
        scoop_id: None,
    },
    PopularApp {
        name: "CCleaner",
        winget_id: Some("Piriform.CCleaner"),
        chocolatey_id: Some("ccleaner"),
        scoop_id: None,
    },
];

/// Packages shown when Browse is opened with an empty search.
/// Prefer winget IDs when that provider is enabled (same authority as Updates).
pub fn list_popular(
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
) -> Vec<Package> {
    let mut packages = Vec::with_capacity(POPULAR.len());

    for app in POPULAR {
        let chosen = if include_winget {
            app.winget_id.map(|id| (ProviderKind::Winget, id))
        } else {
            None
        }
        .or_else(|| {
            if include_chocolatey {
                app.chocolatey_id.map(|id| (ProviderKind::Chocolatey, id))
            } else {
                None
            }
        })
        .or_else(|| {
            if include_scoop {
                app.scoop_id.map(|id| (ProviderKind::Scoop, id))
            } else {
                None
            }
        });

        let Some((provider, pkg_id)) = chosen else {
            continue;
        };

        packages.push(Package {
            id: package_id(provider, pkg_id),
            provider,
            name: app.name.to_string(),
            version: None,
            available_version: None,
            summary: None,
            category: None,
            source: Some("popular".into()),
            pinned: false,
            outdated: false,
        });
    }

    packages
}
