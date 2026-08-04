//! Command surface for the throwaway on-device spike,
//! [#17](https://github.com/Furizaa/poe-graft/issues/17).
//!
//! Two jobs and no more: turn `poe-graft-win32`'s plain structs into serde DTOs, and give macOS
//! stubs so `pnpm tauri dev` still opens a window on the development machine. The DTOs live here
//! rather than in the platform crate for the same reason `PlatformInfoDto` does — that crate
//! stays free of `serde`, which is the seam doing its job.
//!
//! The whole surface is six commands. The frontend sends the entire configuration whenever any
//! part of it changes, so there is one way in rather than nine.

use serde::{Deserialize, Serialize};

/// What the non-Windows stubs say. The development machine has no game, no hook and no
/// clipboard worth poisoning.
#[cfg(not(windows))]
const UNSUPPORTED: &str = "the spike only runs on Windows — this is the macOS stub";

// ---------------------------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------------------------

/// Everything the panel needs, in one poll.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeStatusDto {
    /// False on macOS, where every other field is meaningless.
    pub supported: bool,
    pub hook_installed: bool,
    pub armed: bool,
    pub learning: bool,
    pub suppress: bool,
    pub release_shift: bool,
    pub guard_foreground: bool,
    pub trigger_vk: u32,
    pub trigger_name: String,
    pub last_key_vk: u32,
    pub last_key_name: String,
    /// Physical key-downs the hook callback has observed since it was installed. The one number
    /// that separates "the hook is deaf" from "the panel is not showing what it heard".
    pub keys_seen: u32,
    /// The captured item position, absent until the first press after arming.
    pub position: Option<[i32; 2]>,
    pub rolls: u32,
    pub max_rolls: u32,
    pub copy_delay_ms: u32,
    pub read_timeout_ms: u32,
    pub tolerance_px: i32,
    /// How many unreadable reads in a row disarm the spike. Was a hard-coded 3, which fired
    /// constantly on a jewel that was perfectly fine.
    pub bad_limit: u32,
    /// Physical trigger presses seen while armed, across all arming sessions. Compare against
    /// `rolls`: a growing gap is fail-closed sequencing refusing to queue work.
    pub presses: u32,
    pub shift_down: bool,
    pub foreground: String,
    pub last_roll: Option<RollRecordDto>,
    pub accessibility: Option<AccessibilityDto>,
}

/// One completed roll.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollRecordDto {
    pub roll: u32,
    pub copy_ms: u32,
    pub cycle_ms: u32,
    pub timed_out: bool,
    pub stale: bool,
    pub identical_to_previous: bool,
    pub shift_down: bool,
    pub chars: usize,
    pub summary: String,
}

/// The accessibility settings that change how a held modifier behaves.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessibilityDto {
    pub sticky_keys_on: bool,
    pub sticky_keys_available: bool,
    pub filter_keys_on: bool,
    pub toggle_keys_on: bool,
}

/// The whole configuration, sent as one payload.
///
/// The macOS stub takes this and does nothing with it, which reads as dead code on the
/// development machine only — on Windows every field is used.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeConfigDto {
    pub trigger_vk: u32,
    pub learning: bool,
    pub suppress: bool,
    pub release_shift: bool,
    pub guard_foreground: bool,
    pub copy_delay_ms: u32,
    pub read_timeout_ms: u32,
    pub tolerance_px: i32,
    pub max_rolls: u32,
    pub bad_limit: u32,
}

// ---------------------------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::{AccessibilityDto, RollRecordDto, SpikeConfigDto, SpikeStatusDto};
    use poe_graft_win32::spike;

    pub fn status() -> SpikeStatusDto {
        let status = spike::status();
        SpikeStatusDto {
            supported: true,
            hook_installed: status.hook_installed,
            armed: status.armed,
            learning: status.learning,
            suppress: status.suppress,
            release_shift: status.release_shift,
            guard_foreground: status.guard_foreground,
            trigger_vk: status.trigger_vk,
            trigger_name: status.trigger_name,
            last_key_vk: status.last_key_vk,
            last_key_name: status.last_key_name,
            keys_seen: status.keys_seen,
            position: status.position.map(|(x, y)| [x, y]),
            rolls: status.rolls,
            max_rolls: status.max_rolls,
            copy_delay_ms: status.copy_delay_ms,
            read_timeout_ms: status.read_timeout_ms,
            tolerance_px: status.tolerance_px,
            bad_limit: status.bad_limit,
            presses: status.presses,
            shift_down: status.shift_down,
            foreground: status.foreground,
            last_roll: status.last_roll.map(|roll| RollRecordDto {
                roll: roll.roll,
                copy_ms: roll.copy_ms,
                cycle_ms: roll.cycle_ms,
                timed_out: roll.timed_out,
                stale: roll.stale,
                identical_to_previous: roll.identical_to_previous,
                shift_down: roll.shift_down,
                chars: roll.chars,
                summary: roll.summary,
            }),
            accessibility: spike::accessibility().ok().map(|state| AccessibilityDto {
                sticky_keys_on: state.sticky_keys_on,
                sticky_keys_available: state.sticky_keys_available,
                filter_keys_on: state.filter_keys_on,
                toggle_keys_on: state.toggle_keys_on,
            }),
        }
    }

    pub fn hook(on: bool) -> Result<(), String> {
        if on {
            spike::install().map_err(|err| err.to_string())
        } else {
            spike::uninstall().map_err(|err| err.to_string())
        }
    }

    pub fn arm(on: bool) -> Result<(), String> {
        spike::set_armed(on).map_err(|err| err.to_string())
    }

    pub fn configure(config: SpikeConfigDto) {
        spike::set_trigger(config.trigger_vk);
        spike::set_learning(config.learning);
        spike::set_suppress(config.suppress);
        spike::set_release_shift(config.release_shift);
        spike::set_guard_foreground(config.guard_foreground);
        spike::set_timing(
            config.copy_delay_ms,
            config.read_timeout_ms,
            config.tolerance_px,
            config.max_rolls,
            config.bad_limit,
        );
    }

    pub fn forget_position() {
        spike::forget_position();
    }

    pub fn note(line: &str) {
        spike::note(line);
    }
}

// ---------------------------------------------------------------------------------------------
// Everywhere else
// ---------------------------------------------------------------------------------------------

#[cfg(not(windows))]
mod imp {
    use super::{SpikeConfigDto, SpikeStatusDto, UNSUPPORTED};

    pub fn status() -> SpikeStatusDto {
        SpikeStatusDto {
            supported: false,
            trigger_name: "none".into(),
            last_key_name: "none".into(),
            foreground: "(not Windows)".into(),
            ..Default::default()
        }
    }

    pub fn hook(_on: bool) -> Result<(), String> {
        Err(UNSUPPORTED.to_string())
    }

    pub fn arm(_on: bool) -> Result<(), String> {
        Err(UNSUPPORTED.to_string())
    }

    pub fn configure(_config: SpikeConfigDto) {}

    pub fn forget_position() {}

    pub fn note(_line: &str) {}
}

// ---------------------------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------------------------

/// Everything about the spike's current state, polled by the panel.
#[tauri::command]
pub fn spike_status() -> SpikeStatusDto {
    imp::status()
}

/// Install or remove the `WH_KEYBOARD_LL` hook.
#[tauri::command]
pub fn spike_hook(on: bool) -> Result<(), String> {
    imp::hook(on)
}

/// Arm or disarm. Arming resets the roll count and forgets the captured position.
#[tauri::command]
pub fn spike_arm(on: bool) -> Result<(), String> {
    imp::arm(on)
}

/// Push the whole configuration down at once.
#[tauri::command]
pub fn spike_configure(config: SpikeConfigDto) {
    imp::configure(config);
}

/// Forget the captured item position so the next press recaptures it.
#[tauri::command]
pub fn spike_forget_position() {
    imp::forget_position();
}

/// Write an observation into the log from the frontend, so the human's own notes land in the
/// same file as the machine's — which is the file that survives the updater's force-exit.
#[tauri::command]
pub fn spike_note(line: String) {
    imp::note(&line);
}
