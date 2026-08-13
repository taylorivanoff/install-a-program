use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const CONFIG_ARG_PREFIX: &str = "--tauri-config-file=";
const DEV_SIDECAR_NAME: &str = "install-a-program-tauri-dev.json";

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain([0]).collect()
}

fn dev_sidecar_path() -> PathBuf {
    std::env::temp_dir().join(DEV_SIDECAR_NAME)
}

fn quote_for_powershell(value: &str) -> String {
    value.replace('\'', "''")
}

/// Apply dev config before Tauri starts (elevated relaunch during `bun start` / `tauri dev`).
pub fn apply_dev_config_from_args() {
    for arg in std::env::args().skip(1) {
        if let Some(raw) = arg.strip_prefix(CONFIG_ARG_PREFIX) {
            let path = Path::new(raw.trim_matches('"'));
            if let Ok(config) = std::fs::read_to_string(path) {
                std::env::set_var("TAURI_CONFIG", config);
            }
            let _ = std::fs::remove_file(path);
            return;
        }
    }

    #[cfg(debug_assertions)]
    if std::env::var("TAURI_CONFIG").is_err() {
        if let Ok(config) = std::fs::read_to_string(dev_sidecar_path()) {
            std::env::set_var("TAURI_CONFIG", config);
        }
    }
}

fn write_config_sidecar_for_relaunch() -> Option<PathBuf> {
    let config = std::env::var("TAURI_CONFIG")
        .ok()
        .or_else(|| std::fs::read_to_string(dev_sidecar_path()).ok())?;
    let path = std::env::temp_dir().join(format!(
        "install-a-program-elevate-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, config).ok()?;
    Some(path)
}

fn launch_elevated_deferred(exe: &Path, config_path: Option<&Path>) -> Result<(), String> {
    let exe_str = quote_for_powershell(&exe.display().to_string());
    let ps_command = if let Some(path) = config_path {
        let arg = format!(
            "{}\"{}\"",
            CONFIG_ARG_PREFIX,
            path.display()
        );
        let arg_str = quote_for_powershell(&arg);
        format!(
            "Start-Sleep -Milliseconds 700; Start-Process -FilePath '{exe_str}' -ArgumentList '{arg_str}' -Verb RunAs"
        )
    } else {
        format!("Start-Sleep -Milliseconds 700; Start-Process -FilePath '{exe_str}' -Verb RunAs")
    };

    let params = format!("-NoProfile -WindowStyle Hidden -Command \"{ps_command}\"");

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide("open").as_ptr(),
            wide("powershell.exe").as_ptr(),
            wide(&params).as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    let code = result as isize;
    if code <= 32 {
        return Err(format!(
            "Could not schedule elevated relaunch (ShellExecute error {code})"
        ));
    }

    Ok(())
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        let _ = CloseHandle(token);

        ok != 0 && elevation.TokenIsElevated != 0
    }
}

pub fn request_elevation() -> Result<(), String> {
    if is_elevated() {
        return Ok(());
    }

    let exe =
        std::env::current_exe().map_err(|e| format!("Could not resolve executable: {e}"))?;
    let config_path = write_config_sidecar_for_relaunch();

    // Exit before the elevated process starts so single-instance + dev reload work.
    launch_elevated_deferred(&exe, config_path.as_deref())
}
