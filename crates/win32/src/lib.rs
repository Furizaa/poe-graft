//! Windows plumbing for poe-graft: the real implementation of `poe-graft-core`'s `Platform`, and the
//! executor for the roll cycle.
//!
//! The whole crate is gated on Windows, so on macOS it compiles to nothing. That is deliberate — it
//! is the compile-time half of the seam.
//!
//! [`cycle`] is where the `unsafe` lives: the `WH_KEYBOARD_LL` hook, `SendInput`, and the
//! poison-and-poll clipboard protocol. It decides nothing. Every judgement — whether to Roll, whether
//! to Refuse, whether to Halt — belongs to `poe_graft_core::CraftSession`, which this crate feeds
//! events and takes commands from. That is what makes the cycle testable on a machine with no game on
//! it ([ADR 0002](../../../docs/adr/0002-roll-cycle-and-hit-latch.md)).
#![cfg(windows)]

pub mod cycle;

use poe_graft_core::{Platform, PlatformError, PlatformInfo};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};

/// The Windows platform.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPlatform;

impl WindowsPlatform {
    /// Create the Windows platform.
    pub const fn new() -> Self {
        Self
    }
}

impl Platform for WindowsPlatform {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn info(&self) -> Result<PlatformInfo, PlatformError> {
        // SAFETY: both calls are pure reads of global desktop state. `GetSystemMetrics` takes a
        // value and returns a value. `GetCursorPos` writes two `i32`s into a `POINT` we own and
        // keep alive across the call; it cannot write anywhere else.
        let (width, height) =
            unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };

        let mut point = POINT::default();
        unsafe { GetCursorPos(&mut point) }.map_err(|e| PlatformError::Os {
            capability: "GetCursorPos",
            detail: e.message(),
        })?;

        Ok(PlatformInfo {
            screen: (width, height),
            cursor: (point.x, point.y),
        })
    }
}
