//! The roll cycle: a pure state machine, events in and commands out.
//!
//! This is [ADR 0002](../../../docs/adr/0002-roll-cycle-and-hit-latch.md) in code. There is no
//! clock here, no Win32 and no I/O: a [`CraftSession`] is handed [`Event`]s and hands back an
//! [`Outcome`] holding the [`Command`]s to execute, what to make a noise about, and what to write
//! down. `poe-graft-win32` executes the commands and reports what happened.
//!
//! The reason for that shape is not purity for its own sake. The spike held its state in atomics
//! and slept inline, which works and is proven — but it is testable only on the gaming PC, the one
//! machine that cannot run tests. Here, every path is a sequence of events in
//! `cargo test -p poe-graft-core`, including the ones nobody wants to reproduce by hand: the
//! wrong-item `Halt`, a run of `Unknown` Verdicts, the stale Read that reads perfectly and lies.
//!
//! # The invariant
//!
//! **The app never spends an Alteration without a fresh Miss for the item in front of it.**
//!
//! [`Command::Click`] is the only command that spends one, and it is emitted from exactly one place:
//! a Trigger Press served in [`State::Ready`], which is reachable only from a Read that returned
//! [`Verdict::Miss`]. `no_reachable_path_clicks_without_a_fresh_miss` in `tests/cycle.rs` asserts
//! that over the whole reachable state space rather than trusting this paragraph.
//!
//! # Where the Refusal checks live
//!
//! Shift, the foreground window and the cursor's drift off the Anchor are *read* by `win32` and
//! *judged* here — they arrive as fields on [`Press`]. ADR 0002 put the guard "on the action, in the
//! worker"; the reading stays there, and the decision moved so that every Refusal path is testable
//! on the development machine. The hook callback still touches nothing but atomics, which is the
//! rule that mattered.

use std::sync::Arc;

use crate::item::{parse_item_text, Item, ItemIdentity};
use crate::pool::ModPool;
use crate::verdict::{assess, Diagnostic, Target, Verdict};

/// Timings and bounds, measured on device rather than guessed.
///
/// These ship as code defaults, not as advice in a document: the app the owner runs is whatever the
/// updater delivered, so a tuned number that lives in prose is a number nobody is using.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleConfig {
    /// Between the injected click and `Ctrl+C`, so the copy reads the *new* roll.
    ///
    /// **130 ms.** Applying an Alteration is server-authoritative, and at 40 ms roughly a third of
    /// reads found the Sentinel untouched — the game was not slow to copy, it was not *ready* to.
    /// This is the floor on the roll rate and it is the game's, not ours.
    pub settle_ms: u32,
    /// How long to wait for the clipboard to change before calling the Read a failure. **150 ms**,
    /// against reads that our own code completes in 1–8 ms.
    pub read_timeout_ms: u32,
    /// How long to keep retrying the read after the clipboard sequence number moves. **80 ms**,
    /// because `EmptyClipboard` bumps that number and Path of Exile calls it *before*
    /// `SetClipboardData`.
    pub read_settle_ms: u32,
    /// How far the cursor may drift from the Anchor before a press is Refused. **24 px.**
    pub anchor_tolerance_px: i32,
    /// How many consecutive `Unknown` Verdicts end the Craft Session. **3** — tighter than the
    /// spike's 5, and it can afford to be, because a Resync spends no orb.
    pub unknown_limit: u32,
    /// The window class a Roll requires in the foreground. `POEWindowClass`, measured on device.
    pub game_window_class: String,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            settle_ms: 130,
            read_timeout_ms: 150,
            read_settle_ms: 80,
            anchor_tolerance_px: 24,
            unknown_limit: 3,
            game_window_class: "POEWindowClass".into(),
        }
    }
}

/// Which of ADR 0002's seven states a Craft Session is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// No Craft Session. The Trigger Key is not suppressed and the app cannot click.
    Idle,
    /// Armed, with no Anchor and no Verdict. The next press captures the Anchor and Reads.
    Sighting,
    /// Holding a fresh Miss. The next press Rolls and Reads.
    Ready,
    /// A Roll and its Read are in flight. Further presses are counted and dropped, never queued.
    Rolling,
    /// Holding an `Unknown` Verdict. The next press Resyncs: a Read with no Roll.
    Resyncing,
    /// A Hit has been found. Awaiting acknowledgement; survives losing focus.
    Latched,
    /// Stopped and untrusting. Awaiting re-arm.
    Halted,
}

impl State {
    /// The word the state badge shows.
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Sighting => "Sighting",
            Self::Ready => "Ready",
            Self::Rolling => "Rolling",
            Self::Resyncing => "Resyncing",
            Self::Latched => "Latched",
            Self::Halted => "Halted",
        }
    }

    /// Is a Craft Session under way at all? This is exactly the suppression flag: `Idle` leaves the
    /// Trigger Key alone system-wide, and every other state swallows it.
    pub fn armed(self) -> bool {
        !matches!(self, Self::Idle)
    }
}

/// One thing for `win32` to do. A plan is a list of these, executed in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Write the Sentinel to the clipboard, so anything else found there could only have come from
    /// the game. Content alone can never prove freshness — two identical rolls produce identical
    /// bytes.
    Poison {
        /// The unique text to write.
        sentinel: String,
    },
    /// One injected left-click, wherever the cursor already is. **This is the orb.** No move flag
    /// and no coordinates, so the cursor never moves.
    Click,
    /// Wait, so the client can apply the orb and rebuild the tooltip before it is copied.
    Settle {
        /// Milliseconds.
        ms: u32,
    },
    /// One injected `Ctrl+C`, with Shift left held.
    SendCopy,
    /// Wait for the clipboard to stop holding the Sentinel, then read it.
    AwaitRead {
        /// Give up after this long and report that the game never copied.
        timeout_ms: u32,
        /// Keep retrying the read for this long past the sequence-number bump.
        settle_ms: u32,
    },
}

/// One physical, non-repeating press of the Trigger Key, with what the world looked like when it
/// arrived.
///
/// The three facts are read by `win32` in the worker — never in the hook callback — and judged here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Press {
    /// Was Shift physically held? Without it a click picks the jewel up instead of applying the orb.
    pub shift_down: bool,
    /// The foreground window's class name.
    pub foreground_class: String,
    /// Where the cursor was, in virtual-screen coordinates.
    pub cursor: (i32, i32),
}

/// What a Read produced. Four of the five ways this can go mean the same thing: the app does not
/// know what the item is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    /// The clipboard held something that was not the Sentinel.
    Text(String),
    /// The Sentinel was still there when the timeout expired — the game never copied.
    NothingCopied,
    /// The clipboard changed but nothing readable arrived inside the read-settle window.
    NothingReadable,
    /// An OS call refused, so the Read could not be attempted or completed.
    Failed(String),
}

/// What executing one plan produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleReport {
    /// Did the injected click actually land? **One orb is gone if it did**, whatever the Read then
    /// did or failed to do. Reported rather than assumed, so the Roll count cannot drift from what
    /// was really spent.
    pub rolled: bool,
    /// What the Read came back with.
    pub read: ReadOutcome,
}

/// Something that happened to a Craft Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The human clicked Arm in poe-graft's own window.
    Arm,
    /// The human clicked Stop.
    Disarm,
    /// The human acknowledged a Hit — a mouse click, and the only thing that releases a Latch.
    Acknowledge,
    /// A physical Trigger Press.
    Press(Press),
    /// A plan finished executing.
    CycleFinished(CycleReport),
    /// Presses that arrived while a cycle was in flight and were dropped rather than queued.
    PressesDropped {
        /// How many.
        count: u32,
    },
}

/// A noise to make. Which sounds these are is
/// [#9](https://github.com/Furizaa/poe-graft/issues/9)'s; that they are distinct **in kind** is ADR
/// 0002's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feedback {
    /// Loud and unmistakable. The one event the app exists for.
    Hit,
    /// A warning. The app cannot trust what it is looking at.
    Halt,
    /// A quiet blip meaning "that press did not Roll".
    ///
    /// Not polish: a Resync press is physically identical to a Roll press and the human cannot see
    /// that the item did not change, so without this they would believe they had rolled.
    Blip,
}

/// Why a press was Refused: a precondition of a Roll that is momentarily false.
///
/// Nothing is spent and nothing is learned, and the state does not change — the human fixes it in a
/// second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    /// Shift is not held, so a click would pick the jewel up rather than apply the orb.
    ShiftNotHeld,
    /// Path of Exile is not the foreground window.
    NotTheGame {
        /// The class name of whatever is in front instead.
        foreground_class: String,
    },
    /// The cursor has drifted off the Anchor.
    OffAnchor {
        /// Where the cursor is now.
        cursor: (i32, i32),
        /// Where the jewel was when the session started.
        anchor: (i32, i32),
        /// How far it is allowed to drift.
        tolerance: i32,
    },
}

impl std::fmt::Display for RefusalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShiftNotHeld => write!(
                f,
                "Shift is not held, so a click would pick the jewel up rather than apply the orb. \
                 Hold Shift with the Orbs of Alteration in your inventory."
            ),
            Self::NotTheGame { foreground_class } => write!(
                f,
                "Path of Exile is not the foreground window — {foreground_class} is. Click into the \
                 game, then press again."
            ),
            Self::OffAnchor {
                cursor,
                anchor,
                tolerance,
            } => write!(
                f,
                "the cursor is {}, {} px off the Anchor at {},{} (tolerance {tolerance} px). Hover \
                 the jewel again.",
                (cursor.0 - anchor.0).abs(),
                (cursor.1 - anchor.1).abs(),
                anchor.0,
                anchor.1,
            ),
        }
    }
}

/// Why a Craft Session stopped and needs a deliberate re-arm.
///
/// A Halt means the app cannot trust its own eyes, which is a different thing from a Refusal.
#[derive(Debug, Clone, PartialEq)]
pub enum HaltReason {
    /// The Item Text describes something other than the item this session began on.
    ///
    /// A Halt rather than an `Unknown` precisely because Resyncing would re-read the same wrong item
    /// forever, turning every press into a dead one while telling the human nothing.
    WrongItem {
        /// What the session pinned on its baseline Read.
        expected: String,
        /// What this Read found.
        found: String,
    },
    /// Too many consecutive `Unknown` Verdicts. Usually the orb has left the cursor, or the jewel is
    /// no longer hovered.
    UnknownRun {
        /// How many in a row.
        count: u32,
    },
    /// The hit test saw something that means the app cannot trust `data/ghastly-eye-jewel.json` or
    /// its own reading of the item.
    Untrustworthy {
        /// What it saw.
        diagnostics: Vec<Diagnostic>,
    },
}

impl std::fmt::Display for HaltReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongItem { expected, found } => write!(
                f,
                "the Item Text describes a different item than this Craft Session began on — \
                 expected {expected}, read {found}."
            ),
            Self::UnknownRun { count } => write!(
                f,
                "{count} Reads in a row came back with an Unknown Verdict. The app cannot see what \
                 state the game is in, so it has stopped rather than keep clicking. Check that the \
                 jewel is still hovered, that an Orb of Alteration is still on the cursor, and that \
                 Shift is still held."
            ),
            Self::Untrustworthy { diagnostics } => {
                write!(f, "the Read cannot be trusted:")?;
                for diagnostic in diagnostics {
                    write!(f, " {diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

/// What a Craft Session wants done about one event.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    plan: Vec<Command>,
    feedback: Option<Feedback>,
    log: Vec<String>,
}

impl Outcome {
    /// The commands to execute, in order. Empty means do nothing at all.
    pub fn plan(&self) -> &[Command] {
        &self.plan
    }

    /// The noise to make, if any.
    pub fn feedback(&self) -> Option<Feedback> {
        self.feedback
    }

    /// Lines for the log file.
    ///
    /// Everything the app learns has to reach the **file**, not just the window: the updater
    /// force-exits the app on Windows, and a crashing hook takes the window with it
    /// ([#11](https://github.com/Furizaa/poe-graft/issues/11)).
    pub fn log(&self) -> &[String] {
        &self.log
    }

    fn say(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
    }
}

/// One item, one Target Mod, one continuous attempt.
///
/// Holds the whole cycle. `win32` owns exactly one of these and feeds it events; `src-tauri` reads
/// it for the window.
#[derive(Debug, Clone)]
pub struct CraftSession {
    pool: Arc<ModPool>,
    config: CycleConfig,
    target: Target,
    state: State,
    anchor: Option<(i32, i32)>,
    /// What the baseline Read pinned. Every later Read is compared against it.
    pinned: Option<ItemIdentity>,
    /// The last Item Text seen, so an identical one after a Roll can be caught.
    previous_text: Option<String>,
    /// Is a plan out with `win32` right now?
    ///
    /// A [`CycleReport`] that nobody asked for is discarded. Without this a duplicated report would
    /// double-count an Alteration that was only spent once, and one arriving after a `Disarm` would
    /// drag a closed session back to life — which is the same shape as the spike's worst moment,
    /// where it announced `DISARMED ITSELF` and then rolled 51 ms later.
    awaiting_report: bool,
    rolls: u32,
    consecutive_unknown: u32,
    presses_dropped: u32,
    sentinel_seq: u32,
    last_verdict: Option<Verdict>,
    last_tier: Option<u8>,
    halt: Option<HaltReason>,
    message: String,
}

impl CraftSession {
    /// A session in `Idle`, which cannot click.
    ///
    /// The app comes up here, which is also why the Latch needs no persistence across restarts.
    pub fn new(pool: Arc<ModPool>, target: Target, config: CycleConfig) -> Self {
        Self {
            pool,
            config,
            target,
            state: State::Idle,
            anchor: None,
            pinned: None,
            previous_text: None,
            awaiting_report: false,
            rolls: 0,
            consecutive_unknown: 0,
            presses_dropped: 0,
            sentinel_seq: 0,
            last_verdict: None,
            last_tier: None,
            halt: None,
            message: "Idle. Choose a Target Mod and arm to begin.".into(),
        }
    }

    /// Advance the cycle.
    pub fn handle(&mut self, event: Event) -> Outcome {
        match event {
            Event::Arm => self.arm(),
            Event::Disarm => self.disarm(),
            Event::Acknowledge => self.acknowledge(),
            Event::Press(press) => self.press(press),
            Event::CycleFinished(report) => self.finished(report),
            Event::PressesDropped { count } => self.dropped(count),
        }
    }

    // ── What the window reads ──────────────────────────────────────────────────────────────────

    /// Which state the session is in — the badge.
    pub fn state(&self) -> State {
        self.state
    }

    /// The most recent thing worth saying, in the same words the log got.
    ///
    /// Core owns this copy rather than the frontend so that the window and the file cannot disagree
    /// about why the app stopped.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The Target Mod and Tier Threshold this session is crafting for.
    pub fn target(&self) -> &Target {
        &self.target
    }

    /// The timings and bounds this session runs on.
    ///
    /// Read-only on purpose: these are measured defaults that ship in code, and a window that lets
    /// the human retune the settle delay is a window that lets them retune it *wrong* mid-craft.
    pub fn config(&self) -> &CycleConfig {
        &self.config
    }

    /// Choose a different Target Mod. Only possible while `Idle` — changing it mid-session would
    /// invalidate the Verdict the session is holding.
    pub fn set_target(&mut self, target: Target) -> bool {
        if self.state != State::Idle {
            return false;
        }
        self.target = target;
        true
    }

    /// Alterations spent by this Craft Session.
    pub fn rolls(&self) -> u32 {
        self.rolls
    }

    /// Where the jewel is, once a baseline Read has captured it.
    pub fn anchor(&self) -> Option<(i32, i32)> {
        self.anchor
    }

    /// How many `Unknown` Verdicts in a row. Reaching [`CycleConfig::unknown_limit`] Halts.
    pub fn consecutive_unknown(&self) -> u32 {
        self.consecutive_unknown
    }

    /// Presses that arrived while a cycle was in flight and were dropped rather than queued.
    ///
    /// The spike's equivalent number is the only reason anyone could tell fail-closed sequencing
    /// from a bug, so it survives into the real app.
    pub fn presses_dropped(&self) -> u32 {
        self.presses_dropped
    }

    /// What the last completed Read established.
    pub fn last_verdict(&self) -> Option<Verdict> {
        self.last_verdict
    }

    /// The tier the last Read's numbers implied, when the Target Mod was present and unambiguous.
    pub fn last_tier(&self) -> Option<u8> {
        self.last_tier
    }

    /// Why the session Halted, if it did.
    pub fn halt_reason(&self) -> Option<&HaltReason> {
        self.halt.as_ref()
    }

    /// Should the hook swallow the Trigger Key?
    ///
    /// On whenever a Craft Session is armed, and only then. Scoping it to the foreground window
    /// instead would mean calling `GetForegroundWindow` inside the hook callback, which the
    /// callback's rules forbid; this is one relaxed atomic load, which they permit.
    pub fn suppresses_trigger(&self) -> bool {
        self.state.armed()
    }

    /// A compact description of everything the next decision depends on.
    ///
    /// Deliberately excludes the counters that only ever get reported — Rolls, dropped presses — so
    /// two sessions that will behave identically compare equal. `tests/cycle.rs` uses it to walk the
    /// reachable state space exhaustively instead of sampling event sequences.
    pub fn fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.previous_text.hash(&mut hasher);
        self.pinned
            .as_ref()
            .map(describe_identity)
            .hash(&mut hasher);

        format!(
            "{:?} verdict {:?} anchor {:?} unknown {} awaiting {} target {}@{} read {:016x}",
            self.state,
            self.last_verdict,
            self.anchor,
            self.consecutive_unknown,
            self.awaiting_report,
            self.target.group_id(),
            self.target.tier_threshold(),
            hasher.finish(),
        )
    }

    // ── Arming ─────────────────────────────────────────────────────────────────────────────────

    fn arm(&mut self) -> Outcome {
        let mut out = Outcome::default();

        // Arm and Acknowledge are both mouse clicks in our own window, so Arm must not become a
        // second way to throw a Hit away by misclicking.
        if self.state == State::Latched {
            self.message =
                "A Hit is Latched. Acknowledge it before starting another Craft Session.".into();
            out.say(self.message.clone());
            return out;
        }

        self.reset();
        self.state = State::Sighting;
        // Load-bearing copy, not polish: arming is a mouse click in *our* window, and the hook is
        // deaf while our window has focus ([#18](https://github.com/Furizaa/poe-graft/issues/18)).
        // A human who arms and presses without clicking into the game sees nothing happen at all.
        self.message = format!(
            "Armed for {} at Tier {} or better. Click into Path of Exile with Shift NOT held — a \
             Shift-click while an orb is on the cursor would apply it to whatever you clicked. Then \
             hold Shift, hover the jewel, and tap the Trigger Key once: the first press Reads the \
             jewel and spends no Alteration.",
            self.target.group_id(),
            self.target.tier_threshold(),
        );
        out.say(format!(
            "──── armed · target {} · Tier Threshold {} · settle {}ms · read timeout {}ms · \
             read settle {}ms · Anchor tolerance {}px · Unknown limit {} ────",
            self.target.group_id(),
            self.target.tier_threshold(),
            self.config.settle_ms,
            self.config.read_timeout_ms,
            self.config.read_settle_ms,
            self.config.anchor_tolerance_px,
            self.config.unknown_limit,
        ));
        out.say(self.message.clone());
        out
    }

    fn disarm(&mut self) -> Outcome {
        let mut out = Outcome::default();
        out.say(format!("──── disarmed after {} Roll(s) ────", self.rolls));
        self.reset();
        self.state = State::Idle;
        self.message = "Idle. The Trigger Key is no longer suppressed.".into();
        out.say(self.message.clone());
        out
    }

    fn acknowledge(&mut self) -> Outcome {
        let mut out = Outcome::default();

        if self.state != State::Latched {
            self.message = "Nothing to acknowledge.".into();
            out.say(self.message.clone());
            return out;
        }

        out.say(format!(
            "Hit acknowledged after {} Roll(s). Closing the Craft Session.",
            self.rolls
        ));

        // The realistic next action after a Hit is the next jewel, not the same one — so the target
        // survives and everything about the item does not. Hover the same jewel and the baseline
        // Read immediately re-Latches instead of rolling it.
        self.reset();
        self.state = State::Sighting;
        self.message = format!(
            "New Craft Session for {} at Tier {} or better. Click into Path of Exile with Shift NOT \
             held, hover the next jewel, then hold Shift and tap the Trigger Key once.",
            self.target.group_id(),
            self.target.tier_threshold(),
        );
        out.say(self.message.clone());
        out
    }

    /// Everything about *this* item and *this* attempt. The Target Mod and the config survive.
    fn reset(&mut self) {
        self.anchor = None;
        self.pinned = None;
        self.previous_text = None;
        self.awaiting_report = false;
        self.rolls = 0;
        self.consecutive_unknown = 0;
        self.presses_dropped = 0;
        self.last_verdict = None;
        self.last_tier = None;
        self.halt = None;
        // `sentinel_seq` deliberately keeps counting. It exists so a stale Read can never look like
        // a fresh one, and rewinding it would let a leftover clipboard value do exactly that.
    }

    // ── Presses ────────────────────────────────────────────────────────────────────────────────

    fn press(&mut self, press: Press) -> Outcome {
        let mut out = Outcome::default();

        match self.state {
            State::Idle => {
                // The hook only counts presses while armed, so this is defensive.
                out.say("Trigger Press ignored — no Craft Session is armed.");
                return out;
            }
            State::Rolling => {
                self.presses_dropped += 1;
                out.say(format!(
                    "Trigger Press dropped — a cycle is still in flight ({} dropped this session). \
                     Never queued: a queued press is a second Alteration.",
                    self.presses_dropped
                ));
                return out;
            }
            State::Latched => {
                out.say(
                    "Trigger Press ignored — a Hit is Latched. Only a mouse click in poe-graft's \
                     own window releases it; no key can.",
                );
                out.feedback = Some(Feedback::Blip);
                return out;
            }
            State::Halted => {
                out.say(
                    "Trigger Press ignored — the Craft Session has Halted. Re-arm to continue.",
                );
                out.feedback = Some(Feedback::Blip);
                return out;
            }
            State::Sighting | State::Ready | State::Resyncing => {}
        }

        if let Some(reason) = self.refusal(&press) {
            self.message = format!("Refused: {reason}");
            out.say(format!("Refused a Trigger Press: {reason}"));
            out.feedback = Some(Feedback::Blip);
            return out;
        }

        match self.state {
            State::Sighting => {
                // The only way to capture the Anchor: the cursor has to be over the jewel in the
                // game, so it cannot also be over a button in our window.
                self.anchor = Some(press.cursor);
                out.say(format!(
                    "Anchor captured at {},{}. Reading the jewel — no Roll.",
                    press.cursor.0, press.cursor.1
                ));
                out.plan = self.read_plan();
            }
            State::Resyncing => {
                out.say(
                    "Resyncing — one Ctrl+C, no click, no Alteration. Authoritative by \
                         construction, because no Roll intervened.",
                );
                out.plan = self.read_plan();
                out.feedback = Some(Feedback::Blip);
            }
            State::Ready => {
                out.say(format!(
                    "Rolling on Roll {} — one Shift+left-click on the Anchor, then one Ctrl+C after \
                     {}ms.",
                    self.rolls + 1,
                    self.config.settle_ms
                ));
                out.plan = self.roll_plan();
                self.state = State::Rolling;
                self.message = "Rolling…".into();
            }
            _ => unreachable!("the states that do not serve a press returned above"),
        }

        self.awaiting_report = !out.plan.is_empty();
        out
    }

    /// Is a precondition of this press momentarily false?
    ///
    /// Only `Ready` can Click, so only `Ready` needs Shift. Every state that Reads needs the game in
    /// front, because an injected `Ctrl+C` copies from whatever has focus — a Read taken out of our
    /// own window would be nonsense. And a Read taken with the cursor off the Anchor is a Read of
    /// something else, so the drift guard applies as soon as there is an Anchor to drift from.
    fn refusal(&self, press: &Press) -> Option<RefusalReason> {
        if press.foreground_class != self.config.game_window_class {
            return Some(RefusalReason::NotTheGame {
                foreground_class: press.foreground_class.clone(),
            });
        }

        if self.state == State::Ready && !press.shift_down {
            return Some(RefusalReason::ShiftNotHeld);
        }

        if let Some(anchor) = self.anchor {
            let tolerance = self.config.anchor_tolerance_px;
            if (press.cursor.0 - anchor.0).abs() > tolerance
                || (press.cursor.1 - anchor.1).abs() > tolerance
            {
                return Some(RefusalReason::OffAnchor {
                    cursor: press.cursor,
                    anchor,
                    tolerance,
                });
            }
        }

        None
    }

    fn sentinel(&mut self) -> String {
        let sentinel = format!("poe-graft-sentinel-{}", self.sentinel_seq);
        self.sentinel_seq += 1;
        sentinel
    }

    /// A Read and nothing else: no click, no orb.
    fn read_plan(&mut self) -> Vec<Command> {
        vec![
            Command::Poison {
                sentinel: self.sentinel(),
            },
            Command::SendCopy,
            Command::AwaitRead {
                timeout_ms: self.config.read_timeout_ms,
                settle_ms: self.config.read_settle_ms,
            },
        ]
    }

    /// One Alteration, then a Read of what it produced.
    fn roll_plan(&mut self) -> Vec<Command> {
        vec![
            Command::Poison {
                sentinel: self.sentinel(),
            },
            Command::Click,
            Command::Settle {
                ms: self.config.settle_ms,
            },
            Command::SendCopy,
            Command::AwaitRead {
                timeout_ms: self.config.read_timeout_ms,
                settle_ms: self.config.read_settle_ms,
            },
        ]
    }

    fn dropped(&mut self, count: u32) -> Outcome {
        let mut out = Outcome::default();
        self.presses_dropped += count;
        out.say(format!(
            "{count} Trigger Press(es) dropped while a cycle was in flight ({} this session).",
            self.presses_dropped
        ));
        out
    }

    // ── Results ────────────────────────────────────────────────────────────────────────────────

    fn finished(&mut self, report: CycleReport) -> Outcome {
        let mut out = Outcome::default();

        // Nothing was in flight, so this report describes a cycle this session no longer owns —
        // most likely one the human disarmed out from under. Counting it would spend an orb twice.
        if !self.awaiting_report {
            out.say(format!(
                "Discarded a cycle report ({}, {:?}) — no plan was in flight.",
                if report.rolled { "rolled" } else { "no Roll" },
                report.read,
            ));
            return out;
        }
        self.awaiting_report = false;

        if report.rolled {
            // Counted because the click landed, not because a result came back. The orb is gone
            // whether or not anything is learned afterwards.
            self.rolls += 1;
            out.say(format!("Roll {} — one Alteration spent.", self.rolls));
        }

        let text = match report.read {
            ReadOutcome::Text(text) => text,
            ReadOutcome::NothingCopied => return self.unknown(
                out,
                "the game never copied — the Sentinel was still in the clipboard when the Read \
                     timed out",
            ),
            ReadOutcome::NothingReadable => return self.unknown(
                out,
                "the clipboard changed but nothing readable arrived. `EmptyClipboard` bumps the \
                     sequence number before the text is set, so this is the retry window expiring",
            ),
            ReadOutcome::Failed(detail) => {
                return self.unknown(out, &format!("the Read could not be completed: {detail}"))
            }
        };

        // A fresh clipboard does not mean a fresh item. The Sentinel proves the *clipboard* changed
        // after we poisoned it; it cannot prove the Item Text describes the state *after* the Roll.
        // Copy too early and the game hands back a clean, parseable, confident description of the
        // item as it was before the orb landed — the one remaining path to a silent over-roll.
        //
        // Gated on `rolled` on purpose: after a Resync no Roll intervened, so identical bytes are
        // authoritative rather than suspicious. Without that gate a Resync could never resolve.
        let stale = report.rolled && self.previous_text.as_deref() == Some(text.as_str());
        self.previous_text = Some(text.clone());
        if stale {
            return self.unknown(
                out,
                "the Item Text is byte-identical to the previous Roll's, so it is probably a \
                 photograph taken before the game finished applying the orb",
            );
        }

        let item = match parse_item_text(&text, &self.pool) {
            Ok(item) => item,
            Err(unreadable) => {
                return self.unknown(out, &format!("the Item Text will not parse: {unreadable}"))
            }
        };

        match self.pinned.as_ref().map(|pinned| pinned == item.identity()) {
            Some(false) => {
                let expected =
                    describe_identity(self.pinned.as_ref().expect("just matched as Some"));
                let found = describe_identity(item.identity());
                return self.halt(out, HaltReason::WrongItem { expected, found });
            }
            None => {
                // The baseline Read. `ItemIdentity` deliberately excludes `Requirements: Level:`,
                // which moves with whichever mods rolled.
                let described = describe_identity(item.identity());
                self.pinned = Some(item.identity().clone());
                out.say(format!(
                    "Pinned {described}. Every later Read is compared against it."
                ));
            }
            Some(true) => {}
        }

        self.judge(out, &item)
    }

    fn judge(&mut self, mut out: Outcome, item: &Item) -> Outcome {
        let assessment = assess(item, &self.target, &self.pool);

        // Every Diagnostic is logged, whether or not it changes anything — a wrong row in the tier
        // data has to be discoverable rather than invisible.
        for diagnostic in assessment.diagnostics() {
            out.say(format!("Diagnostic: {diagnostic}"));
        }

        if assessment.halt_worthy() {
            return self.halt(
                out,
                HaltReason::Untrustworthy {
                    diagnostics: assessment.diagnostics().to_vec(),
                },
            );
        }

        self.last_tier = assessment.tier();

        match assessment.verdict() {
            Verdict::Hit => {
                self.last_verdict = Some(Verdict::Hit);
                self.consecutive_unknown = 0;
                self.state = State::Latched;
                self.message = format!(
                    "HIT — {} at Tier {} after {} Roll(s). Latched: the next press will not Roll. \
                     Acknowledge with the mouse when you have taken the jewel off the cursor.",
                    self.target.group_id(),
                    assessment
                        .tier()
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "unknown — ambiguous, failing closed".into()),
                    self.rolls,
                );
                out.say(self.message.clone());
                out.feedback = Some(Feedback::Hit);
            }
            Verdict::Miss => {
                self.last_verdict = Some(Verdict::Miss);
                self.consecutive_unknown = 0;
                self.state = State::Ready;
                self.message = match assessment.tier() {
                    Some(tier) => format!(
                        "Miss — {} rolled Tier {tier}, worse than Tier {}. Press to Roll.",
                        self.target.group_id(),
                        self.target.tier_threshold()
                    ),
                    None => format!(
                        "Miss — {} is not on the jewel. Press to Roll.",
                        self.target.group_id()
                    ),
                };
                out.say(self.message.clone());
            }
            Verdict::Unknown => {
                return self.unknown(out, "the Read established nothing about the Target Mod");
            }
        }

        out
    }

    /// Nothing trustworthy came back. This is the state ADR 0002 exists to make cheap: the next
    /// press Resyncs, spending a press rather than an orb.
    fn unknown(&mut self, mut out: Outcome, why: &str) -> Outcome {
        self.last_verdict = Some(Verdict::Unknown);
        self.last_tier = None;
        self.consecutive_unknown += 1;

        out.say(format!(
            "Unknown Verdict ({} in a row): {why}.",
            self.consecutive_unknown
        ));

        if self.consecutive_unknown >= self.config.unknown_limit {
            let count = self.consecutive_unknown;
            return self.halt(out, HaltReason::UnknownRun { count });
        }

        self.state = State::Resyncing;
        self.message = format!(
            "Unknown — poe-graft has lost track of the jewel ({} of {} before it stops). Press \
             again to Resync: that press Reads without Rolling and spends no Alteration.",
            self.consecutive_unknown, self.config.unknown_limit
        );
        out.say(self.message.clone());
        out
    }

    fn halt(&mut self, mut out: Outcome, reason: HaltReason) -> Outcome {
        // A Halted session establishes nothing, so it holds no Verdict either — which is also what
        // makes it impossible for a stale Miss to survive a Halt and authorise a Click afterwards.
        self.last_verdict = None;
        self.last_tier = None;
        self.state = State::Halted;
        self.message = format!("HALTED — {reason} Re-arm to continue.");
        self.halt = Some(reason);
        out.say(self.message.clone());
        out.feedback = Some(Feedback::Halt);
        out
    }
}

/// What a Craft Session pins, in words — for the log, the window, and the wrong-item Halt.
fn describe_identity(identity: &ItemIdentity) -> String {
    format!(
        "{} {} ({}, Item Level {})",
        identity.rarity(),
        identity.base_name(),
        identity.item_class(),
        identity.item_level(),
    )
}
