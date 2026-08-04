//! Platform-independent core for poe-graft.
//!
//! This crate cannot name a Win32 symbol — that is the seam, and it is enforced by the
//! compiler rather than by convention (see `docs/adr/0001-stack-and-seam.md`). Everything
//! the app needs from the operating system arrives through [`Platform`].
//!
//! The vocabulary is [`CONTEXT.md`](../../../CONTEXT.md) and the cycle it serves is
//! [ADR 0002](../../../docs/adr/0002-roll-cycle-and-hit-latch.md). What is here so far is the
//! **mod pool**, the **Item Text parser** and the **hit test** — everything needed to turn a Read
//! into a Verdict. The roll cycle's state machine lands next
//! ([#20](https://github.com/Furizaa/poe-graft/issues/20)).

pub mod item;
pub mod platform;
pub mod pool;
pub mod verdict;

pub use item::{parse_item_text, Annotation, Item, ItemIdentity, ParsedMod, Rarity, Unreadable};
pub use platform::{Platform, PlatformError, PlatformInfo, StubPlatform};
pub use pool::{Band, DataError, Generation, ModGroup, ModPool, ModTier};
pub use verdict::{assess, Assessment, Diagnostic, Target, Verdict};
