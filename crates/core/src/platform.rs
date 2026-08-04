//! The operating-system seam.
//!
//! `Platform` is the whole surface the core is allowed to reach the OS through. Windows
//! implements it for real in `poe-graft-win32`; macOS gets [`StubPlatform`], which exists so
//! `pnpm tauri dev` runs on the development machine.
//!
//! The three capability methods named in ADR 0001 — hook, inject, clipboard — are **not** on
//! this trait yet. They are shaped by the roll cycle, which
//! [Design the roll cycle and the hit latch](https://github.com/Furizaa/poe-graft/issues/7)
//! settles. What is here is the part the bootstrap genuinely needs: enough of a real Win32
//! call to prove the `windows` crate compiles and links in the release build on CI.

use std::fmt;

/// Why a platform call could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    /// This platform does not implement the capability at all. macOS returns this for
    /// everything; it is the expected answer on the development machine, not a bug.
    Unsupported {
        /// The capability that was asked for, for the log line.
        capability: &'static str,
    },
    /// The OS was asked and refused, or answered with something unusable.
    Os {
        /// What was being attempted.
        capability: &'static str,
        /// The OS's own description, already stringified — the core never sees an error code.
        detail: String,
    },
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { capability } => {
                write!(f, "{capability} is not supported on this platform")
            }
            Self::Os { capability, detail } => write!(f, "{capability} failed: {detail}"),
        }
    }
}

impl std::error::Error for PlatformError {}

/// A liveness readout from the platform layer.
///
/// Deliberately boring. Its only job in the bootstrap is to be something the app can show on
/// the gaming PC that could only have come from a real Win32 call, so a green CI build is
/// distinguishable from a build where the native layer silently did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformInfo {
    /// Primary display size in pixels.
    pub screen: (i32, i32),
    /// Current cursor position in virtual-screen coordinates.
    pub cursor: (i32, i32),
}

/// Everything poe-graft is allowed to ask the operating system for.
pub trait Platform: Send + Sync {
    /// A short name for logs: `"windows"`, `"stub"`.
    fn name(&self) -> &'static str;

    /// Read a liveness readout from the OS.
    fn info(&self) -> Result<PlatformInfo, PlatformError>;
}

/// The macOS (and anything-not-Windows) implementation: refuses everything, politely.
///
/// This is what makes the development loop work — the app runs, the window opens, and every
/// platform call reports `Unsupported` rather than panicking or pretending to succeed.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubPlatform;

impl StubPlatform {
    /// Create the stub platform.
    pub const fn new() -> Self {
        Self
    }
}

impl Platform for StubPlatform {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn info(&self) -> Result<PlatformInfo, PlatformError> {
        Err(PlatformError::Unsupported {
            capability: "platform info",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_usable_behind_dyn() {
        // The app holds a `Box<dyn Platform>` chosen at compile time, so the trait has to be
        // object-safe. If this stops compiling, the seam has stopped being a seam.
        let platform: Box<dyn Platform> = Box::new(StubPlatform::new());
        assert_eq!(platform.name(), "stub");
    }

    #[test]
    fn stub_reports_unsupported_rather_than_failing_silently() {
        let err = StubPlatform::new().info().unwrap_err();
        assert_eq!(
            err,
            PlatformError::Unsupported {
                capability: "platform info"
            }
        );
        // The human on a machine with no debugger reads this string, so it has to say
        // something.
        assert!(err.to_string().contains("not supported"));
    }
}
