# Install a Program

Pick apps from a Ninite-style Browse grid and install them in one run on Windows or search packages from winget, Chocolatey or Scoop, then keep your PC updated from the **Updates** tab.

<img width="1102" height="792" alt="{23CDA1FF-E880-44D5-BEE4-D85B3D361237}" src="https://github.com/user-attachments/assets/53b0ee9b-1d63-4aa0-bff1-4d49a6fbdf77" />

<img width="1102" height="792" alt="{7964AE6D-992A-400A-91FC-6D36CBF5E471}" src="https://github.com/user-attachments/assets/2ec0abfd-19f5-49d0-adf7-2cfa2ae707ea" />

<img width="1102" height="792" alt="{EFD7F402-4A37-4A55-8B35-25135E5C2E42}" src="https://github.com/user-attachments/assets/4b48aff4-4c7d-4de9-bffe-ca142803469c" />

## Features

- **Browse grid** — category cards for popular apps (Ninite-style picker)
- **Presets** — Fresh PC, Developer, Gaming, Student one-click bundles
- **Simple mode** (default) — winget-first, minimal UI; Advanced mode unlocks all tabs and providers
- **Conflict rules** — one browser, PDF reader, or archiver at a time
- **Copy / paste bundle** — share a JSON selection via clipboard
- **Standalone installer** — export a single `.exe` that silently installs your selection via winget (no GUI required on the target PC)
- **Portable build** — run the full app without the NSIS installer
- **Setup complete** — post-install summary with shortcut to Updates
- **Bulk update** — update-all from the Updates view; filter by Chocolatey, winget, or Scoop (even in Simple mode)
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

Build the standalone runner template before exporting installers locally:

```bash
npm run build:runner
mkdir -p src-tauri/resources
cp src-tauri/target/release/install-a-program-runner.exe src-tauri/resources/runner-template.exe
```

## Build

NSIS installer:

```bash
npm run build
```

Portable app (no installer):

```bash
npm run build:portable
```

Output: `src-tauri/target/release/Install a Program (Bulk Edition).exe`

NSIS installer output:

`src-tauri/target/release/bundle/nsis/Install a Program (Bulk Edition)_0.1.2_x64-setup.exe`

### Standalone bundle runner

1. Select apps in Browse
2. Click **Save standalone installer…**
3. Run the exported `.exe` on another PC that has **winget** (App Installer)

The runner installs only winget packages. Chocolatey/Scoop packages are not supported in standalone exports. The runner template is embedded in the app at build time — use **Save standalone installer…** to produce a working `.exe`; the bare template is not published as a release download.

## SEO / discovery keywords

install multiple programs, ninite alternative, bulk install software, chocolatey gui, winget gui, windows package manager, scoop gui, silent install, bulk update, uninstall programs
