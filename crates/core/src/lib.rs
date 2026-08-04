//! Platform-independent core for poe-graft.
//!
//! This crate cannot name a Win32 symbol — that is the seam, and it is enforced by the
//! compiler rather than by convention (see `docs/adr/0001-stack-and-seam.md`). Everything
//! the app needs from the operating system arrives through [`Platform`].
//!
//! The vocabulary is [`CONTEXT.md`](../../../CONTEXT.md) and the cycle it serves is
//! [ADR 0002](../../../docs/adr/0002-roll-cycle-and-hit-latch.md). Four pieces, in the order a roll
//! passes through them: the **mod pool** ([`pool`]), the **Item Text parser** ([`item`]), the
//! **hit test** ([`verdict`]), and the **roll cycle** itself ([`cycle`]) — a state machine with no
//! clock and no I/O, so the whole thing replays on the development machine.

pub mod cycle;
pub mod item;
pub mod platform;
pub mod pool;
pub mod verdict;

pub use cycle::{
    Command, CraftSession, CycleConfig, CycleReport, Event, Feedback, HaltReason, Outcome, Press,
    ReadOutcome, RefusalReason, State,
};
pub use item::{parse_item_text, Annotation, Item, ItemIdentity, ParsedMod, Rarity, Unreadable};
pub use platform::{Platform, PlatformError, PlatformInfo, StubPlatform};
pub use pool::{Band, DataError, Generation, ModGroup, ModPool, ModTier};
pub use verdict::{assess, Assessment, Diagnostic, Target, Verdict};
