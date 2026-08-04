//! Tauri wiring. Deliberately thin: it picks a `Platform`, loads the tier data, opens a log, starts
//! the roll cycle, and exposes all of that to the webview. No domain logic lives here — see
//! `docs/adr/0001-stack-and-seam.md`.

mod build_info;
mod cycle;
mod journal;
mod pool;

use build_info::BuildInfo;
use journal::Journal;
#[cfg(windows)]
use poe_graft_core::{CraftSession, CycleConfig};
use poe_graft_core::{ModPool, Platform, Target};
use pool::ModPoolDto;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Manager, State};

// Only the `cfg(windows)` startup path builds a Craft Session, but the target defaults below stay
// compiled everywhere on purpose: `cargo check -p poe-graft` runs on the Mac and `cargo check -p
// poe-graft --target x86_64-pc-windows-msvc` does not, so anything excluded here is code CI is the
// first to compile.
/// The Mod Group the map is aimed at — `Minions deal # to # additional Physical Damage` — and the tier
/// that makes the acceptance test. The default, not a constraint: the window can pick any group.
#[cfg_attr(not(windows), allow(dead_code))]
const DEFAULT_TARGET_GROUP: &str = "MinionAddedPhysicalDamage";
#[cfg_attr(not(windows), allow(dead_code))]
const DEFAULT_TIER_THRESHOLD: u8 = 1;

/// Everything the commands need, resolved once at startup.
struct AppState {
    build: BuildInfo,
    journal: Arc<Journal>,
    platform: Box<dyn Platform>,
    /// The tier data. `None` means it failed to load, and then nothing can arm — there is no
    /// fallback table and there must never be one.
    pool: Option<Arc<ModPool>>,
    /// Where the tier data came from, or why it did not.
    pool_source: String,
}

/// The Windows implementation of the seam.
#[cfg(windows)]
fn platform() -> Box<dyn Platform> {
    Box::new(poe_graft_win32::WindowsPlatform::new())
}

/// Everywhere else — the development machine. Reports `Unsupported` for everything, which is what lets
/// `pnpm tauri dev` run on macOS.
#[cfg(not(windows))]
fn platform() -> Box<dyn Platform> {
    Box::new(poe_graft_core::StubPlatform::new())
}

/// `PlatformInfo` on the wire. The core crate has no serde dependency, so the DTO lives here — which is
/// the seam doing its job rather than an inconvenience.
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
            state
                .journal
                .append(&format!("platform info failed: {message}"));
            Err(message)
        }
    }
}

/// The tier data, for the Target Mod picker. An error here means the app cannot craft at all.
#[tauri::command]
fn mod_pool(state: State<'_, AppState>) -> Result<ModPoolDto, String> {
    match &state.pool {
        Some(pool) => Ok(ModPoolDto::of(pool)),
        None => Err(state.pool_source.clone()),
    }
}

/// Where the tier data came from. Shown in the window, because "which file is this actually running?"
/// has no other answer on a machine with no dev environment.
#[tauri::command]
fn mod_pool_source(state: State<'_, AppState>) -> String {
    state.pool_source.clone()
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

/// Let the frontend write into the same file. The updater's whole story — every check, every version
/// the feed reported, every raw error — arrives this way, which is the point: it has to outlive the
/// window it was displayed in.
#[tauri::command]
fn log_append(state: State<'_, AppState>, line: String) {
    state.journal.append(&line);
}

/// The Target Mod a fresh session starts on: the map's, if this pool has it, and otherwise the first
/// group there is — so a Base whose data does not contain the map's target still comes up usable.
#[cfg_attr(not(windows), allow(dead_code))]
fn default_target(pool: &ModPool) -> Target {
    if pool.group_by_id(DEFAULT_TARGET_GROUP).is_some() {
        return Target::new(DEFAULT_TARGET_GROUP, DEFAULT_TIER_THRESHOLD);
    }
    match pool.groups().first() {
        Some(group) => Target::new(group.id(), 1),
        None => Target::new(DEFAULT_TARGET_GROUP, DEFAULT_TIER_THRESHOLD),
    }
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

            // The hook and worker threads have no access to Tauri state, and what they learn has to
            // outlive the window: the updater force-exits the app, and a panicking hook takes the
            // window with it. So they get a direct line to the same file, installed before anything
            // can produce a finding.
            #[cfg(windows)]
            {
                let sink = Arc::clone(&journal);
                poe_graft_win32::cycle::set_log_sink(Box::new(move |line| sink.append(line)));
            }

            let (pool, pool_source) = match pool::load(app.handle()) {
                Ok((pool, from)) => {
                    journal.append(&format!(
                        "tier data: {} · {} Mod Groups · read from {from}",
                        pool.base_name(),
                        pool.groups().len()
                    ));
                    (Some(pool), from)
                }
                Err(err) => {
                    // Loud, and in the file. An app that came up without its tier data and said
                    // nothing would look exactly like an app that is working.
                    journal.append(&format!(
                        "──── TIER DATA FAILED TO LOAD: {err} · poe-graft cannot assess a Read, so \
                         no Craft Session can start. Check bundle.resources in tauri.conf.json. ────"
                    ));
                    (None, err)
                }
            };

            // Starting the cycle installs the keyboard hook. It does not arm anything — the app comes
            // up `Idle`, and `Idle` cannot click.
            #[cfg(windows)]
            if let Some(pool) = &pool {
                let session = CraftSession::new(
                    Arc::clone(pool),
                    default_target(pool),
                    CycleConfig::default(),
                );
                if let Err(err) = poe_graft_win32::cycle::start(session) {
                    journal.append(&format!(
                        "──── the keyboard hook could not be installed: {err} · the Trigger Key will \
                         do nothing. ────"
                    ));
                }
            }

            app.manage(AppState {
                build,
                journal,
                platform,
                pool,
                pool_source,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            build_info,
            platform_info,
            mod_pool,
            mod_pool_source,
            log_path,
            log_tail,
            log_append,
            cycle::cycle_status,
            cycle::cycle_arm,
            cycle::cycle_acknowledge,
            cycle::cycle_set_target,
            cycle::cycle_set_trigger,
            cycle::cycle_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
