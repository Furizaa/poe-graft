//! Windows plumbing for poe-graft: the real implementation of `poe-graft-core`'s `Platform`.
//!
//! The whole crate is gated on Windows, so on macOS it compiles to nothing. That is deliberate
//! — it is the compile-time half of the seam.
//!
//! The `Platform` impl here is still one real Win32 read and no more — the hook, `SendInput` and
//! the clipboard belong to the roll cycle, and the cycle is not designed yet.
//!
//! [`spike`] is the exception, and it is quarantined on purpose: a throwaway payload for
//! [#17](https://github.com/Furizaa/poe-graft/issues/17) that exercises all three mechanisms once
//! per physical keypress so the cycle can be designed against facts instead of assumptions. It
//! deliberately sits beside the seam rather than inside it — nothing in `poe-graft-core` knows it
//! exists, so it can be deleted whole.
#![cfg(windows)]

pub mod spike;

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
