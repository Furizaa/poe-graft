//! Windows plumbing for poe-graft: the real implementation of `poe-graft-core`'s `Platform`.
//!
//! The whole crate is gated on Windows, so on macOS it compiles to nothing. That is deliberate
//! — it is the compile-time half of the seam.
//!
//! At bootstrap this crate is one real Win32 read and no more. The `WH_KEYBOARD_LL` hook,
//! `SendInput` and the clipboard poison-and-poll all land here later; what exists now is the
//! smallest thing that proves the `windows` crate compiles, links and runs inside a release
//! bundle on the gaming PC — the riskiest unproven part of the toolchain, and the one that is
//! most expensive to discover is broken on a machine with no debugger.
#![cfg(windows)]

use poe_graft_core::{Platform, PlatformError, PlatformInfo};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

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
        let (width, height) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };

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
