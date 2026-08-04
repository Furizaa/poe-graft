//! Command surface for the roll cycle.
//!
//! Two jobs and no more: turn `poe-graft-win32`'s plain structs into serde DTOs, and give the
//! development machine stubs so `pnpm tauri dev` still opens a window on macOS. The DTOs live here
//! rather than in the platform crate for the same reason `PlatformInfoDto` does — that crate stays free
//! of `serde`, which is the seam doing its job.
//!
//! Nothing here decides anything. Every Verdict, every Refusal and every Halt is
//! `poe_graft_core::CraftSession`'s, and the window only ever shows what Rust reports.

use serde::Serialize;

/// What the non-Windows stubs say. The development machine has no game, no hook and no clipboard worth
/// poisoning.
#[cfg(not(windows))]
const UNSUPPORTED: &str =
    "the roll cycle only runs on Windows — this is the macOS stub, and it has no game to talk to";

// ---------------------------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------------------------

/// Everything the window needs, in one poll.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleStatusDto {
    /// False on macOS, where the hook, the injection and the clipboard are all absent.
    pub supported: bool,
    /// Is the hook installed and the worker alive?
    pub running: bool,
    /// The state badge: `Idle`, `Sighting`, `Ready`, `Rolling`, `Resyncing`, `Latched`, `Halted`.
    pub state: String,
    /// The most recent thing worth saying — core's own copy, so the window and the log agree.
    pub message: String,
    pub target_group: String,
    pub tier_threshold: u8,
    pub rolls: u32,
    /// The Anchor, absent until the baseline Read captures it.
    pub anchor: Option<[i32; 2]>,
    /// Presses that arrived mid-cycle and were dropped rather than queued. A growing number is
    /// fail-closed sequencing working, not a bug.
    pub presses_dropped: u32,
    pub consecutive_unknown: u32,
    pub unknown_limit: u32,
    pub last_verdict: Option<String>,
    pub last_tier: Option<u8>,
    /// Why the session Halted. The window shows this verbatim: after a Halt the human needs to read
    /// *why*, and paraphrasing it on a machine with no dev tools would be worse than useless.
    pub halt_reason: Option<String>,
    /// Is the Trigger Key being swallowed right now? On exactly while a Craft Session is armed.
    pub suppress: bool,
    pub trigger_vk: u32,
    pub trigger_name: String,
    /// Physical key-downs the hook callback has observed. The one number that separates "the hook is
    /// deaf" from "the window is not showing what it heard".
    pub keys_seen: u32,
    /// Physical Trigger Presses seen while armed.
    pub presses: u32,
    pub shift_down: bool,
    pub foreground: String,
    /// Monotonic counters. The window plays one sound per increment.
    pub hits: u32,
    pub halts: u32,
    pub blips: u32,
    pub copy_ms: Option<u32>,
    pub cycle_ms: Option<u32>,
    pub accessibility: Option<AccessibilityDto>,
}

/// The accessibility settings that change how a held modifier behaves.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessibilityDto {
    /// Sticky Keys is on. It silently changes what holding Shift means, which breaks Apply Mode with
    /// no error and no visible sign.
    pub sticky_keys_on: bool,
    /// Its five-taps-on-Shift shortcut is enabled — a gesture a Shift-heavy crafting session can trip
    /// by accident.
    pub sticky_keys_available: bool,
    pub filter_keys_on: bool,
    pub toggle_keys_on: bool,
}

// ---------------------------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::{AccessibilityDto, CycleStatusDto};
    use poe_graft_core::Target;
    use poe_graft_win32::cycle;

    pub fn status() -> CycleStatusDto {
        let status = cycle::status();
        CycleStatusDto {
            supported: true,
            running: status.running,
            state: status.state.to_string(),
            message: status.message,
            target_group: status.target_group,
            tier_threshold: status.tier_threshold,
            rolls: status.rolls,
            anchor: status.anchor.map(|(x, y)| [x, y]),
            presses_dropped: status.presses_dropped,
            consecutive_unknown: status.consecutive_unknown,
            unknown_limit: status.unknown_limit,
            last_verdict: status.last_verdict.map(str::to_string),
            last_tier: status.last_tier,
            halt_reason: status.halt_reason,
            suppress: status.suppress,
            trigger_vk: status.trigger_vk,
            trigger_name: status.trigger_name,
            keys_seen: status.keys_seen,
            presses: status.presses,
            shift_down: status.shift_down,
            foreground: status.foreground,
            hits: status.hits,
            halts: status.halts,
            blips: status.blips,
            copy_ms: status.timing.map(|t| t.copy_ms),
            cycle_ms: status.timing.map(|t| t.cycle_ms),
            accessibility: cycle::accessibility().ok().map(|state| AccessibilityDto {
                sticky_keys_on: state.sticky_keys_on,
                sticky_keys_available: state.sticky_keys_available,
                filter_keys_on: state.filter_keys_on,
                toggle_keys_on: state.toggle_keys_on,
            }),
        }
    }

    pub fn arm(on: bool) -> Result<(), String> {
        cycle::arm(on).map_err(|err| err.to_string())
    }

    pub fn acknowledge() {
        cycle::acknowledge();
    }

    pub fn set_target(target: Target) -> Result<(), String> {
        if cycle::set_target(target) {
            Ok(())
        } else {
            Err("the Target Mod cannot change while a Craft Session is armed. Stop first.".into())
        }
    }

    pub fn set_trigger(vk: u32) {
        cycle::set_trigger(vk);
    }

    pub fn note(line: &str) {
        cycle::note(line);
    }
}

// ---------------------------------------------------------------------------------------------
// Everywhere else
// ---------------------------------------------------------------------------------------------

#[cfg(not(windows))]
mod imp {
    use super::{CycleStatusDto, UNSUPPORTED};
    use poe_graft_core::Target;

    pub fn status() -> CycleStatusDto {
        CycleStatusDto {
            supported: false,
            state: "Idle".into(),
            message: UNSUPPORTED.into(),
            trigger_name: "[ { (0xDB)".into(),
            foreground: "(not Windows)".into(),
            ..Default::default()
        }
    }

    pub fn arm(_on: bool) -> Result<(), String> {
        Err(UNSUPPORTED.to_string())
    }

    pub fn acknowledge() {}

    pub fn set_target(_target: Target) -> Result<(), String> {
        Err(UNSUPPORTED.to_string())
    }

    pub fn set_trigger(_vk: u32) {}

    pub fn note(_line: &str) {}
}

// ---------------------------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------------------------

use crate::AppState;
use poe_graft_core::Target;
use tauri::State;

/// Everything about the cycle's current state, polled by the window.
#[tauri::command]
pub fn cycle_status() -> CycleStatusDto {
    imp::status()
}

/// Arm or disarm a Craft Session. Arming is a mouse click in our own window, which is why
/// [#18](https://github.com/Furizaa/poe-graft/issues/18) makes `Sighting`'s copy load-bearing.
#[tauri::command]
pub fn cycle_arm(state: State<'_, AppState>, on: bool) -> Result<(), String> {
    if state.pool.is_none() {
        return Err(
            "there is no tier data loaded, so a Read could not be assessed. See the log.".into(),
        );
    }
    imp::arm(on)
}

/// Acknowledge a Latched Hit. The **only** thing that releases a Latch, and deliberately a mouse
/// click: a key that can clear a Hit is a key your reflexes can clear a Hit with.
#[tauri::command]
pub fn cycle_acknowledge() {
    imp::acknowledge();
}

/// Choose the Target Mod and Tier Threshold. Validated against the loaded pool here, so `win32` never
/// has to hold an opinion about the data.
#[tauri::command]
pub fn cycle_set_target(
    state: State<'_, AppState>,
    group_id: String,
    tier_threshold: u8,
) -> Result<(), String> {
    let Some(pool) = &state.pool else {
        return Err("there is no tier data loaded.".into());
    };
    let Some(group) = pool.group_by_id(&group_id) else {
        return Err(format!("{group_id} is not a Mod Group of this Base."));
    };
    if !group.tiers().iter().any(|t| t.tier() == tier_threshold) {
        return Err(format!(
            "{group_id} has no Tier {tier_threshold} — it has {} tiers.",
            group.tier_count()
        ));
    }
    imp::set_target(Target::new(group_id, tier_threshold))
}

/// Choose the Trigger Key by virtual-key code.
///
/// Typed rather than learned by pressing, because the hook is deaf while our own window has focus —
/// which is exactly when the human would be trying to teach it a key.
#[tauri::command]
pub fn cycle_set_trigger(vk: u32) -> Result<(), String> {
    if vk > 0xFF {
        return Err(format!("{vk} is not a virtual-key code (0–255)."));
    }
    imp::set_trigger(vk);
    Ok(())
}

/// Write one of the human's own observations into the machine's log, in order, timestamped — so
/// "Apply Mode dropped out around Roll 30" lands in the same file as the machine's own account of it.
#[tauri::command]
pub fn cycle_note(line: String) {
    imp::note(&line);
}
