//! Tauri wiring. Deliberately thin: it picks a `Platform`, opens a log, and exposes both to the
//! webview. No domain logic lives here — see `docs/adr/0001-stack-and-seam.md`.

mod build_info;
mod journal;
mod spike;

use build_info::BuildInfo;
use journal::Journal;
use poe_graft_core::Platform;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Manager, State};

/// Everything the commands need, resolved once at startup.
struct AppState {
    build: BuildInfo,
    journal: Arc<Journal>,
    platform: Box<dyn Platform>,
}

/// The Windows implementation of the seam.
#[cfg(windows)]
fn platform() -> Box<dyn Platform> {
    Box::new(poe_graft_win32::WindowsPlatform::new())
}

/// Everywhere else — the development machine. Reports `Unsupported` for everything, which is
/// what lets `pnpm tauri dev` run on macOS.
#[cfg(not(windows))]
fn platform() -> Box<dyn Platform> {
    Box::new(poe_graft_core::StubPlatform::new())
}

/// `PlatformInfo` on the wire. The core crate has no serde dependency, so the DTO lives here —
/// which is the seam doing its job rather than an inconvenience.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfoDto {
    screen_width: i32,
    screen_height: i32,
    cursor_x: i32,
    cursor_y: i32,
}

/// Provenance for the running binary.
#[tauri::command]
fn build_info(state: State<'_, AppState>) -> BuildInfo {
    state.build.clone()
}

/// One real read through the platform seam. On macOS this always fails, on purpose.
#[tauri::command]
fn platform_info(state: State<'_, AppState>) -> Result<PlatformInfoDto, String> {
    match state.platform.info() {
        Ok(info) => {
            state.journal.append(&format!(
                "platform info: screen {}x{}, cursor {},{}",
                info.screen.0, info.screen.1, info.cursor.0, info.cursor.1
            ));
            Ok(PlatformInfoDto {
                screen_width: info.screen.0,
                screen_height: info.screen.1,
                cursor_x: info.cursor.0,
                cursor_y: info.cursor.1,
            })
        }
        Err(err) => {
            let message = err.to_string();
            state.journal.append(&format!("platform info failed: {message}"));
            Err(message)
        }
    }
}

/// The log file's absolute path, so the UI can show it and reveal it in Explorer.
#[tauri::command]
fn log_path(state: State<'_, AppState>) -> String {
    state.journal.path().display().to_string()
}

/// The tail of the log, for the panel in the UI.
#[tauri::command]
fn log_tail(state: State<'_, AppState>) -> Vec<String> {
    state.journal.tail()
}

/// Let the frontend write into the same file. The updater's whole story — every check, every
/// version the feed reported, every raw error — arrives this way, which is the point: it has to
/// outlive the window it was displayed in.
#[tauri::command]
fn log_append(state: State<'_, AppState>, line: String) {
    state.journal.append(&line);
}

/// Build and run the app.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let journal = Arc::new(Journal::new(app.path().app_log_dir()?.join("poe-graft.log")));
            let platform = platform();
            let build = BuildInfo::new(app.package_info().version.to_string(), platform.name());

            journal.append("──── launch ────");
            journal.append(&build.summary());
            if let Some(url) = &build.run_url {
                journal.append(&format!("built by {url}"));
            }

            // The spike's hook and worker threads have no access to Tauri state, and what they
            // learn has to outlive the window: the updater force-exits the app, and a panicking
            // hook takes the window with it. So they get a direct line to the same file.
            #[cfg(windows)]
            {
                let sink = Arc::clone(&journal);
                poe_graft_win32::spike::set_log_sink(Box::new(move |line| sink.append(line)));
            }

            app.manage(AppState {
                build,
                journal,
                platform,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            build_info,
            platform_info,
            log_path,
            log_tail,
            log_append,
            spike::spike_status,
            spike::spike_hook,
            spike::spike_arm,
            spike::spike_configure,
            spike::spike_forget_position,
            spike::spike_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
