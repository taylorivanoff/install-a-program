pub mod bootstrap;
pub mod elevation;
mod programs;

pub use elevation::apply_dev_config_from_args;
mod providers;
pub mod runner;
pub mod standalone;
mod uninstall;

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;
use tauri::AppHandle;
use tauri_tray_base::{
    apply_window_settings, install_state, setup_tray, sync_autostart, with_common_plugins,
    TrayBaseOptions, TrayExtraItem, TraySetupOptions,
};

use providers::{parse_package_id, PackageAction, ProviderKind};

#[tauri::command]
fn check_elevated() -> bool {
    elevation::is_elevated()
}

#[tauri::command]
fn request_elevation(app: AppHandle) -> Result<(), String> {
    elevation::request_elevation()?;
    // Exit so the elevated relaunch is not treated as a second instance.
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn provider_status() -> bootstrap::ProviderStatus {
    bootstrap::provider_status()
}

#[tauri::command]
async fn bootstrap_chocolatey() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(bootstrap::install_chocolatey)
        .await
        .map_err(|e| format!("Bootstrap task failed: {e}"))?
}

#[tauri::command]
async fn ensure_providers(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || bootstrap::ensure_providers(&app))
        .await
        .map_err(|e| format!("Provider bootstrap task failed: {e}"))?
}

#[tauri::command]
async fn ensure_winget(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || bootstrap::ensure_winget(&app))
        .await
        .map_err(|e| format!("winget bootstrap task failed: {e}"))?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportStandaloneRequest {
    dest_path: String,
    name: Option<String>,
    ids: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportStandaloneResult {
    path: String,
    app_count: usize,
}

#[tauri::command]
fn export_standalone_installer(
    request: ExportStandaloneRequest,
) -> Result<ExportStandaloneResult, String> {
    let bundle = standalone::StandaloneBundle::new(request.name, request.ids);
    let path = std::path::PathBuf::from(&request.dest_path);
    let app_count = standalone::export_standalone_installer(&path, &bundle)?;
    Ok(ExportStandaloneResult {
        path: request.dest_path,
        app_count,
    })
}

#[tauri::command]
fn pick_standalone_save_path(default_name: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new().set_title("Save standalone installer");
    if let Some(name) = default_name.filter(|n| !n.trim().is_empty()) {
        dialog = dialog.set_file_name(name);
    } else {
        dialog = dialog.set_file_name("Install-Programs-setup.exe");
    }
    Ok(dialog.save_file().map(|p| p.display().to_string()))
}

#[tauri::command]
async fn list_installed(
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
) -> Result<Vec<providers::Package>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        providers::list_installed(include_chocolatey, include_winget, include_scoop)
    })
    .await
    .map_err(|e| format!("List installed task failed: {e}"))?
}

#[tauri::command]
async fn search_packages(
    query: String,
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
) -> Result<Vec<providers::Package>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        providers::search(&query, include_chocolatey, include_winget, include_scoop)
    })
    .await
    .map_err(|e| format!("Search packages task failed: {e}"))?
}

#[tauri::command]
async fn list_popular_packages(
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
) -> Result<Vec<providers::Package>, String> {
    Ok(tauri::async_runtime::spawn_blocking(move || {
        providers::list_popular(include_chocolatey, include_winget, include_scoop)
    })
    .await
    .map_err(|e| format!("List popular packages task failed: {e}"))?)
}

#[tauri::command]
async fn list_outdated(
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
    prefer_provider: Option<String>,
    show_duplicates: Option<bool>,
) -> Result<Vec<providers::Package>, String> {
    let show_duplicates = show_duplicates.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        providers::list_outdated(
            include_chocolatey,
            include_winget,
            include_scoop,
            prefer_provider,
            show_duplicates,
        )
    })
    .await
    .map_err(|e| format!("List outdated task failed: {e}"))?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionRequest {
    action: PackageAction,
    ids: Vec<String>,
}

#[tauri::command]
async fn run_package_action(app: AppHandle, request: ActionRequest) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if request.ids.is_empty() {
            return Err("No packages selected".into());
        }
        let action = request.action.as_str();
        for id in &request.ids {
            let (kind, name) = match parse_package_id(id) {
                Ok(v) => v,
                Err(err) => {
                    runner::emit_progress(
                        &app,
                        id,
                        id,
                        "failed",
                        Some(err),
                        None,
                        None,
                    );
                    continue;
                }
            };
            let display = name.clone();
            let result = match kind {
                ProviderKind::Chocolatey => {
                    providers::chocolatey::run_action(&app, &name, &display, action)
                }
                ProviderKind::Winget => {
                    providers::winget::run_action(&app, &name, &display, action)
                }
                ProviderKind::Scoop => providers::scoop::run_action(&app, &name, &display, action),
            };
            if let Err(err) = result {
                // Progress already emitted on failure paths; keep batch going.
                let _ = err;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let _ = tauri::Emitter::emit(&app, "package-finished", ());
        Ok(())
    })
    .await
    .map_err(|e| format!("Package action task failed: {e}"))?
}

#[tauri::command]
async fn list_programs(show_system: bool) -> Result<Vec<programs::InstalledProgram>, String> {
    tauri::async_runtime::spawn_blocking(move || programs::list_installed_programs(show_system))
        .await
        .map_err(|e| format!("List programs task failed: {e}"))?
}

#[tauri::command]
async fn uninstall_programs(app: AppHandle, ids: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || uninstall::uninstall_programs(app, ids))
        .await
        .map_err(|e| format!("Uninstall task failed: {e}"))?
}

#[tauri::command]
async fn list_choco_sources() -> Result<Vec<providers::ChocoSource>, String> {
    tauri::async_runtime::spawn_blocking(providers::chocolatey::list_sources)
        .await
        .map_err(|e| format!("List sources task failed: {e}"))?
}

#[tauri::command]
fn add_choco_source(name: String, url: String) -> Result<(), String> {
    providers::chocolatey::add_source(&name, &url)
}

#[tauri::command]
fn remove_choco_source(name: String) -> Result<(), String> {
    providers::chocolatey::remove_source(&name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = with_common_plugins(tauri::Builder::default())
        .invoke_handler(tauri::generate_handler![
            tauri_tray_base::settings_get,
            tauri_tray_base::settings_set,
            tauri_tray_base::app_get_state,
            check_elevated,
            request_elevation,
            provider_status,
            bootstrap_chocolatey,
            ensure_providers,
            ensure_winget,
            export_standalone_installer,
            pick_standalone_save_path,
            list_installed,
            search_packages,
            list_popular_packages,
            list_outdated,
            run_package_action,
            list_programs,
            uninstall_programs,
            list_choco_sources,
            add_choco_source,
            remove_choco_source,
        ])
        .setup(|app| {
            let mut defaults = HashMap::new();
            defaults.insert("alwaysOnTop".into(), json!(false));
            defaults.insert("startMinimised".into(), json!(false));
            defaults.insert("opacity".into(), json!(1.0));
            // Updates: winget wins when the same app is listed by multiple providers.
            defaults.insert("updateAuthority".into(), json!("winget"));
            defaults.insert("showUpdateDuplicates".into(), json!(false));
            defaults.insert("simpleMode".into(), json!(true));

            install_state(
                app.handle(),
                TrayBaseOptions {
                    app_name: "Install Many Programs".into(),
                    settings_file_name: "install-a-program-settings.json".into(),
                    defaults,
                    show_always_on_top: false,
                    extra_tray_items: vec![TrayExtraItem {
                        id: "refresh".into(),
                        label: "Refresh".into(),
                    }],
                    ..Default::default()
                },
            )?;

            setup_tray(app.handle(), TraySetupOptions::default())?;
            apply_window_settings(app.handle());
            tauri_tray_base::enable_frameless_chrome(app.handle());
            sync_autostart(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            tauri_tray_base::on_window_event(window, event);
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running Install Many Programs");
}
