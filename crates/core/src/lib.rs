//! Platform-independent core for poe-graft.
//!
//! This crate cannot name a Win32 symbol — that is the seam, and it is enforced by the
//! compiler rather than by convention (see `docs/adr/0001-stack-and-seam.md`). Everything
//! the app needs from the operating system arrives through [`Platform`].
//!
//! At bootstrap this crate holds only the seam itself. The roll cycle, the item parser, the
//! hit test and the mod model all land here later; their names and states are decided by
//! [Design the roll cycle and the hit latch](https://github.com/Furizaa/poe-graft/issues/7),
//! so nothing here should be read as pre-empting them.

pub mod platform;

pub use platform::{Platform, PlatformError, PlatformInfo, StubPlatform};
