# Install a Program (Bulk Edition)

Install multiple programs at once on Windows — a **Ninite-style bulk installer** and **Chocolatey GUI / winget GUI** alternative.

Search, multi-select, and install packages from **Chocolatey**, **winget**, and **Scoop**. Bulk-update outdated apps, manage Chocolatey sources, bootstrap Chocolatey when it is missing, and uninstall classic Win32/MSI programs from the registry (same workflow as [Uninstall a Program (Bulk Edition)](../bulk-uninstaller)).

## Features

- **Bulk install** — Ninite-alternative flow: search, select many, install in one run
- **Chocolatey + winget + Scoop** in one list UI
- **Updates** view with update-all
- **Programs** view for classic registry uninstall
- **Chocolatey bootstrap** via the official install script (binaries are not redistributed)
- Live activity log streaming CLI output
- Tray, autostart, and close-to-tray via `tauri-tray-base`

## Requirements

- Windows 10/11
- [Rust](https://rustup.rs/), Node or Bun
- Sibling checkout of [`tauri-tray-base`](https://github.com/taylorivanoff/tauri-tray-base) at `../tauri-tray-base` (from this repo: `Projects/tauri-tray-base`)

## Develop

```bash
npm install
# or: bun install
npm run dev
```

## Build

```bash
npm run build
```

Installer output:

`src-tauri/target/release/bundle/nsis/Install a Program (Bulk Edition)_0.1.0_x64-setup.exe`

## SEO / discovery keywords

install multiple programs, ninite alternative, bulk install software, chocolatey gui, winget gui, windows package manager, scoop gui, silent install, bulk update, uninstall programs
