//! The roll cycle: Trigger Presses and read results in, commands out.
//!
//! Every path ADR 0002 draws is exercised here on the development machine, which is the entire
//! reason the cycle is a pure state machine rather than atomics and inline sleeps in `win32`
//! ([#20](https://github.com/Furizaa/poe-graft/issues/20)).
//!
//! The invariant these tests exist to protect: **the app never spends an Alteration without a fresh
//! Miss for the item in front of it.** `no_reachable_path_clicks_without_a_fresh_miss` asserts it
//! over the whole reachable state space rather than trusting the cases below to be complete.

mod common;

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use poe_graft_core::{
    Command, CraftSession, CycleConfig, CycleReport, Event, Feedback, ModPool, Press, ReadOutcome,
    State, Target, Verdict,
};

/// The map's target — `Minions deal # to # additional Physical Damage`.
const TARGET: &str = "MinionAddedPhysicalDamage";

/// Where the jewel is. Any coordinate does; the Anchor is whatever the first press saw.
const ANCHOR: (i32, i32) = (1440, 820);

/// The shipped mod pool, parsed once. Sessions share it, which is also how `src-tauri` holds it.
fn pool() -> Arc<ModPool> {
    Arc::new(common::pool())
}

/// A session armed on the target at `threshold`.
fn session(threshold: u8) -> CraftSession {
    CraftSession::new(
        pool(),
        Target::new(TARGET, threshold),
        CycleConfig::default(),
    )
}

/// A press with every precondition satisfied: Shift held, Path of Exile in front, cursor on the
/// Anchor.
fn press() -> Event {
    Event::Press(Press {
        shift_down: true,
        foreground_class: "POEWindowClass".into(),
        cursor: ANCHOR,
    })
}

/// A press with one precondition broken.
fn press_with(shift_down: bool, class: &str, cursor: (i32, i32)) -> Event {
    Event::Press(Press {
        shift_down,
        foreground_class: class.into(),
        cursor,
    })
}

/// A read that produced Item Text, after a Roll.
fn rolled(text: &str) -> Event {
    Event::CycleFinished(CycleReport {
        rolled: true,
        read: ReadOutcome::Text(text.to_string()),
    })
}

/// A read that produced Item Text with no Roll behind it — a baseline Read or a Resync.
fn read(text: &str) -> Event {
    Event::CycleFinished(CycleReport {
        rolled: false,
        read: ReadOutcome::Text(text.to_string()),
    })
}

/// A Roll whose Read came back with nothing: the game never copied.
fn rolled_blind() -> Event {
    Event::CycleFinished(CycleReport {
        rolled: true,
        read: ReadOutcome::NothingCopied,
    })
}

/// Does this plan spend an Orb of Alteration?
fn spends_an_orb(plan: &[Command]) -> bool {
    plan.contains(&Command::Click)
}

/// Press, then report the Read — telling the session the truth about whether the plan it just handed
/// back actually spent an orb. Returns false if the press was not served at all.
fn press_and_read(session: &mut CraftSession, text: &str) -> bool {
    let plan = session.handle(press()).plan().to_vec();
    if plan.is_empty() {
        return false;
    }
    session.handle(Event::CycleFinished(CycleReport {
        rolled: spends_an_orb(&plan),
        read: ReadOutcome::Text(text.to_string()),
    }));
    true
}

/// Press, then report a Read that came back with nothing.
fn press_and_fail(session: &mut CraftSession) -> bool {
    let plan = session.handle(press()).plan().to_vec();
    if plan.is_empty() {
        return false;
    }
    session.handle(Event::CycleFinished(CycleReport {
        rolled: spends_an_orb(&plan),
        read: ReadOutcome::NothingCopied,
    }));
    true
}

/// The T4 capture of the target group.
fn t4_capture() -> String {
    common::capture("spike-17/05-annealed-of-order.txt")
}

/// A capture without the target group on it at all — cold damage, not physical.
fn missing_capture() -> String {
    common::capture("spike-17/28-glaciated.txt")
}

/// A second capture without the target group, for when two distinct Misses are needed.
fn another_miss() -> String {
    common::capture("spike-17/03-healthy.txt")
}

/// A Ghastly Eye Jewel is not the only thing the cursor can end up over.
fn wrong_item() -> String {
    missing_capture()
        .replace("Glaciated Ghastly Eye Jewel", "Glaciated Cobalt Jewel")
        .replace("Item Class: Abyss Jewels", "Item Class: Jewels")
}

/// Drive a session from `Idle` to `Ready`, holding a fresh Miss for the pinned jewel.
fn ready_session(threshold: u8) -> CraftSession {
    let mut s = session(threshold);
    s.handle(Event::Arm);
    assert!(press_and_read(&mut s, &missing_capture()));
    assert_eq!(
        s.state(),
        State::Ready,
        "a baseline Miss leaves the session Ready"
    );
    s
}

// ── Arming, and the first press ─────────────────────────────────────────────────────────────────

#[test]
fn an_idle_session_cannot_click() {
    let mut s = session(1);

    assert_eq!(s.state(), State::Idle);
    let outcome = s.handle(press());

    assert!(outcome.plan().is_empty(), "Idle emits no commands at all");
    assert_eq!(s.state(), State::Idle);
    assert_eq!(s.rolls(), 0);
}

#[test]
fn the_first_press_of_a_session_reads_without_rolling() {
    let mut s = session(1);
    s.handle(Event::Arm);
    assert_eq!(s.state(), State::Sighting);

    let outcome = s.handle(press());

    // ADR 0002's consequence that falls out of the invariant for free: the app cannot roll an item
    // it has never looked at, including a jewel that already carries the Target Mod.
    assert!(!spends_an_orb(outcome.plan()));
    assert_eq!(
        outcome.plan(),
        &[
            Command::Poison {
                sentinel: "poe-graft-sentinel-0".into()
            },
            Command::SendCopy,
            Command::AwaitRead {
                timeout_ms: 150,
                settle_ms: 80,
            },
        ]
    );
    assert_eq!(s.anchor(), Some(ANCHOR), "and it captures the Anchor");
}

#[test]
fn arming_tells_the_human_to_click_into_the_game_first() {
    let mut s = session(1);

    let outcome = s.handle(Event::Arm);

    // Load-bearing copy, not polish: arming is a mouse click in *our* window, and the hook is deaf
    // while our window has focus ([#18](https://github.com/Furizaa/poe-graft/issues/18)). A human
    // who arms and then presses without clicking into the game sees nothing happen at all.
    assert!(
        s.message().contains("Click into Path of Exile"),
        "{:?}",
        s.message()
    );
    assert!(outcome.log().iter().any(|l| l.contains("armed")));
}

#[test]
fn a_baseline_miss_makes_the_next_press_roll() {
    let mut s = ready_session(4);

    let outcome = s.handle(press());

    assert_eq!(s.state(), State::Rolling);
    assert_eq!(
        outcome.plan(),
        &[
            Command::Poison {
                sentinel: "poe-graft-sentinel-1".into()
            },
            Command::Click,
            Command::Settle { ms: 130 },
            Command::SendCopy,
            Command::AwaitRead {
                timeout_ms: 150,
                settle_ms: 80,
            },
        ],
        "the roll plan is ADR 0002's, with the measured defaults in it"
    );
}

#[test]
fn the_sentinel_is_different_for_every_read() {
    // Two identical rolls produce identical Item Text, so content alone can never prove freshness —
    // which is the whole reason the Sentinel exists, and why reusing one would defeat it.
    let mut s = CraftSession::new(
        pool(),
        Target::new(TARGET, 1),
        CycleConfig {
            unknown_limit: 10,
            ..CycleConfig::default()
        },
    );
    s.handle(Event::Arm);

    let mut seen = Vec::new();
    for _ in 0..4 {
        let outcome = s.handle(press());
        seen.push(
            outcome
                .plan()
                .iter()
                .find_map(|c| match c {
                    Command::Poison { sentinel } => Some(sentinel.clone()),
                    _ => None,
                })
                .expect("every Read poisons the clipboard first"),
        );
        s.handle(Event::CycleFinished(CycleReport {
            rolled: false,
            read: ReadOutcome::NothingCopied,
        }));
    }

    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 4, "two Reads must never share a Sentinel");
}

#[test]
fn the_timings_come_from_configuration_rather_than_being_baked_in() {
    let mut s = CraftSession::new(
        pool(),
        Target::new(TARGET, 1),
        CycleConfig {
            settle_ms: 200,
            read_timeout_ms: 250,
            read_settle_ms: 90,
            ..CycleConfig::default()
        },
    );
    s.handle(Event::Arm);
    press_and_read(&mut s, &missing_capture());

    let outcome = s.handle(press());

    assert!(outcome.plan().contains(&Command::Settle { ms: 200 }));
    assert!(outcome.plan().contains(&Command::AwaitRead {
        timeout_ms: 250,
        settle_ms: 90
    }));
}

// ── The Latch ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_hit_latches_and_sounds() {
    let mut s = ready_session(4);
    let plan = s.handle(press()).plan().to_vec();
    assert!(spends_an_orb(&plan));

    let outcome = s.handle(rolled(&t4_capture()));

    assert_eq!(s.state(), State::Latched);
    assert_eq!(outcome.feedback(), Some(Feedback::Hit));
    assert_eq!(s.last_verdict(), Some(Verdict::Hit));
    assert_eq!(s.last_tier(), Some(4));
    assert_eq!(s.rolls(), 1, "the orb that found it still counts");
}

#[test]
fn no_press_can_release_the_latch() {
    let mut s = ready_session(4);
    press_and_read(&mut s, &t4_capture());
    assert_eq!(s.state(), State::Latched);

    // A key that can clear a Hit is a key your reflexes can clear a Hit with, which would defeat the
    // whole app on the one press that matters. Fifty of them change nothing.
    for _ in 0..50 {
        let outcome = s.handle(press());
        assert!(outcome.plan().is_empty(), "a Latched session emits nothing");
        assert_eq!(s.state(), State::Latched);
    }
    assert_eq!(s.rolls(), 1);
}

#[test]
fn arming_while_latched_is_refused_so_a_misclick_cannot_discard_a_hit() {
    let mut s = ready_session(4);
    press_and_read(&mut s, &t4_capture());

    s.handle(Event::Arm);

    // Arm and Acknowledge are both mouse clicks in our own window, so Arm must not be a second way
    // to throw a Hit away.
    assert_eq!(s.state(), State::Latched);
    assert_eq!(s.rolls(), 1);
}

#[test]
fn acknowledging_opens_a_new_session_in_sighting_with_the_same_target() {
    let mut s = ready_session(4);
    press_and_read(&mut s, &t4_capture());

    s.handle(Event::Acknowledge);

    assert_eq!(s.state(), State::Sighting);
    assert_eq!(s.rolls(), 0, "a new Craft Session counts its own Rolls");
    assert_eq!(
        s.anchor(),
        None,
        "the Anchor is recaptured, never inherited"
    );
    assert_eq!(s.target().group_id(), TARGET);
    assert_eq!(s.target().tier_threshold(), 4);
}

#[test]
fn acknowledging_and_hovering_the_same_jewel_latches_again_rather_than_rolling_it() {
    let mut s = ready_session(4);
    press_and_read(&mut s, &t4_capture());
    s.handle(Event::Acknowledge);

    // ADR 0002's pleasing property: the new session's baseline Read sees the Hit that is still
    // there, so the jewel cannot be rolled past.
    let outcome = s.handle(press());
    assert!(!spends_an_orb(outcome.plan()));
    s.handle(read(&t4_capture()));

    assert_eq!(s.state(), State::Latched);
    assert_eq!(s.rolls(), 0, "and it cost nothing to find out");
}

#[test]
fn disarming_a_latched_session_is_allowed_because_idle_cannot_click() {
    let mut s = ready_session(4);
    press_and_read(&mut s, &t4_capture());

    s.handle(Event::Disarm);

    assert_eq!(s.state(), State::Idle);
    assert!(s.handle(press()).plan().is_empty());
}

// ── Unknown Verdicts, and the Resync ───────────────────────────────────────────────────────────

#[test]
fn an_unknown_verdict_costs_a_press_and_not_an_orb() {
    let mut s = ready_session(4);
    s.handle(press());
    let outcome = s.handle(rolled_blind());

    assert_eq!(s.state(), State::Resyncing);
    assert_eq!(s.last_verdict(), Some(Verdict::Unknown));
    assert_eq!(
        outcome.feedback(),
        None,
        "the blip belongs to the press, not the report"
    );

    // The Resync: one Ctrl+C, no click, no orb — and it is authoritative by construction, because no
    // Roll intervenes between the previous state and this Read.
    let outcome = s.handle(press());
    assert!(!spends_an_orb(outcome.plan()));
    assert_eq!(
        outcome.feedback(),
        Some(Feedback::Blip),
        "a Resync press is physically identical to a Roll press, so it has to say so"
    );

    s.handle(read(&another_miss()));
    assert_eq!(s.state(), State::Ready);
    assert_eq!(s.rolls(), 1, "one orb, for the one click");
}

#[test]
fn identical_text_after_a_roll_is_unknown_because_it_may_predate_the_orb() {
    let mut s = ready_session(4);

    // The one remaining path to a silent over-roll: a copy taken before the server finished applying
    // the orb returns a clean, parseable, confident photograph of the item as it was.
    press_and_read(&mut s, &missing_capture());

    assert_eq!(s.state(), State::Resyncing);
    assert_eq!(s.last_verdict(), Some(Verdict::Unknown));
    assert_eq!(s.rolls(), 1);
}

#[test]
fn identical_text_after_a_resync_is_authoritative() {
    let mut s = ready_session(4);
    s.handle(press());
    s.handle(rolled_blind());
    assert_eq!(s.state(), State::Resyncing);

    // The same bytes as the baseline Read, and this time they mean something: no Roll intervened, so
    // whatever the server settled on is what the tooltip now shows — identical text included.
    // Treating this as stale would make a Resync unable to ever resolve anything.
    press_and_read(&mut s, &missing_capture());

    assert_eq!(s.state(), State::Ready);
    assert_eq!(s.last_verdict(), Some(Verdict::Miss));
}

#[test]
fn three_consecutive_unknowns_halt_the_session() {
    let mut s = ready_session(4);

    press_and_fail(&mut s);
    assert_eq!(s.state(), State::Resyncing);
    press_and_fail(&mut s);
    assert_eq!(s.state(), State::Resyncing);

    let plan = s.handle(press()).plan().to_vec();
    assert!(!spends_an_orb(&plan), "a Resync never spends an orb");
    let outcome = s.handle(Event::CycleFinished(CycleReport {
        rolled: false,
        read: ReadOutcome::NothingCopied,
    }));

    assert_eq!(s.state(), State::Halted);
    assert_eq!(outcome.feedback(), Some(Feedback::Halt));
    // The guard's job is no longer damage control — a run of Unknowns spends no orbs now — it is
    // telling the human that the orb has left the cursor or the jewel is no longer hovered.
    assert!(s.halt_reason().is_some());
    assert_eq!(
        s.rolls(),
        1,
        "and the whole run cost one orb, for the one click"
    );
}

#[test]
fn a_recovered_read_resets_the_unknown_run() {
    let mut s = ready_session(4);

    press_and_fail(&mut s);
    press_and_fail(&mut s);
    assert_eq!(s.consecutive_unknown(), 2);

    press_and_read(&mut s, &another_miss());
    assert_eq!(s.state(), State::Ready);
    assert_eq!(s.consecutive_unknown(), 0);

    // Two more Unknowns must not tip a counter that should have gone back to zero.
    press_and_fail(&mut s);
    press_and_fail(&mut s);
    assert_eq!(
        s.state(),
        State::Resyncing,
        "not Halted — the run was broken"
    );
}

#[test]
fn a_run_of_unknowns_spends_no_orbs() {
    let mut s = ready_session(4);

    // In the spike each unreadable Read burned an orb. This is the accident the guard exists for —
    // the orb leaving the cursor — costing nothing while it is detected.
    s.handle(press());
    s.handle(rolled_blind());
    for _ in 0..2 {
        let plan = s.handle(press()).plan().to_vec();
        assert!(!spends_an_orb(&plan));
        s.handle(Event::CycleFinished(CycleReport {
            rolled: false,
            read: ReadOutcome::NothingReadable,
        }));
    }

    assert_eq!(s.state(), State::Halted);
    assert_eq!(s.rolls(), 1);
}

#[test]
fn every_kind_of_failed_read_is_an_unknown_verdict() {
    // Four ways to end a Read with nothing trustworthy, and ADR 0002 makes them all one thing: the
    // app does not know what the item is.
    for read in [
        ReadOutcome::NothingCopied,
        ReadOutcome::NothingReadable,
        ReadOutcome::Failed("SendInput injected 0 of 2 events".into()),
        ReadOutcome::Text("this is not item text".into()),
    ] {
        let mut s = ready_session(4);
        s.handle(press());
        s.handle(Event::CycleFinished(CycleReport { rolled: true, read }));

        assert_eq!(s.state(), State::Resyncing);
        assert_eq!(s.last_verdict(), Some(Verdict::Unknown));
    }
}

// ── Halts ──────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_wrong_item_halts_rather_than_resyncing_on_it_forever() {
    let mut s = ready_session(4);

    // A different Base under the Anchor. Resyncing would re-read it forever, turning every press
    // into a dead one while telling the human nothing.
    press_and_read(&mut s, &wrong_item());

    assert_eq!(s.state(), State::Halted);
    assert!(
        format!("{}", s.halt_reason().expect("a reason")).contains("different item"),
        "{}",
        s.halt_reason().expect("a reason")
    );
}

#[test]
fn a_different_item_level_is_the_wrong_item_too() {
    let mut s = ready_session(4);

    press_and_read(
        &mut s,
        &missing_capture().replace("Item Level: 83", "Item Level: 84"),
    );

    assert_eq!(s.state(), State::Halted);
}

#[test]
fn a_requirements_section_that_moves_with_the_roll_is_not_the_wrong_item() {
    let mut s = ready_session(4);

    // `Requirements: Level:` tracks whichever mods rolled — 46 on the baseline, 43 here — and one
    // capture has no `Requirements:` section at all. Pinning it would make every Roll look like a
    // different item.
    press_and_read(&mut s, &t4_capture());

    assert_eq!(
        s.state(),
        State::Latched,
        "the same jewel, differently rolled"
    );
}

#[test]
fn a_tier_data_disagreement_halts_because_our_own_data_is_wrong() {
    let mut s = ready_session(4);

    // The numbers say T4; the game says T2. The Verdict still comes from the numbers, but the app
    // has just learned it cannot trust `data/ghastly-eye-jewel.json`, so it stops.
    press_and_read(&mut s, &t4_capture().replace("(Tier: 4)", "(Tier: 2)"));

    assert_eq!(s.state(), State::Halted);
}

#[test]
fn an_unrecognised_line_halts() {
    let mut s = ready_session(4);

    // Either the game's wording changed or the pool is missing a mod. Either way the app is reading
    // an item it does not understand, and a mod it fails to recognise is exactly how a Hit gets
    // misread as a Miss.
    press_and_read(
        &mut s,
        &missing_capture().replace(
            "Minions deal 20(20-23) to 26(26-32) additional Cold Damage",
            "Minions deal 20 to 26 additional Sonic Damage",
        ),
    );

    assert_eq!(s.state(), State::Halted);
}

#[test]
fn a_halted_session_ignores_presses_until_it_is_re_armed() {
    let mut s = ready_session(4);
    press_and_read(&mut s, &wrong_item());
    assert_eq!(s.state(), State::Halted);

    for _ in 0..5 {
        assert!(s.handle(press()).plan().is_empty());
        assert_eq!(s.state(), State::Halted);
    }

    s.handle(Event::Arm);
    assert_eq!(s.state(), State::Sighting);
    assert_eq!(s.rolls(), 0);
    assert!(
        s.halt_reason().is_none(),
        "re-arming clears the reason it stopped"
    );
}

// ── Refusals ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn shift_up_refuses_because_a_click_would_pick_the_jewel_up() {
    let mut s = ready_session(4);

    let outcome = s.handle(press_with(false, "POEWindowClass", ANCHOR));

    assert!(outcome.plan().is_empty());
    assert_eq!(outcome.feedback(), Some(Feedback::Blip));
    assert_eq!(s.state(), State::Ready, "a Refusal changes no state at all");
    assert_eq!(s.rolls(), 0);
    assert!(s.message().contains("Shift"), "{:?}", s.message());
}

#[test]
fn another_window_in_the_foreground_refuses() {
    let mut s = ready_session(4);

    let outcome = s.handle(press_with(true, "Chrome_WidgetWin_1", ANCHOR));

    assert!(outcome.plan().is_empty());
    assert_eq!(s.state(), State::Ready);
    assert!(
        s.message().contains("Chrome_WidgetWin_1"),
        "the log has to name what was in front instead: {:?}",
        s.message()
    );
}

#[test]
fn a_cursor_off_the_anchor_refuses_but_a_small_drift_does_not() {
    let mut s = ready_session(4);

    let inside = s.handle(press_with(
        true,
        "POEWindowClass",
        (ANCHOR.0 + 24, ANCHOR.1 - 24),
    ));
    assert!(
        spends_an_orb(inside.plan()),
        "24 px is the tolerance, inclusive"
    );
    s.handle(rolled(&another_miss()));
    assert_eq!(s.state(), State::Ready);

    let outside = s.handle(press_with(
        true,
        "POEWindowClass",
        (ANCHOR.0 + 25, ANCHOR.1),
    ));
    assert!(outside.plan().is_empty());
    assert_eq!(s.state(), State::Ready);
}

#[test]
fn a_refusal_does_not_count_towards_the_unknown_limit() {
    let mut s = ready_session(4);

    // A Refusal is a momentarily false precondition the human fixes in a second, and it establishes
    // nothing — so it must not push a session towards a Halt it has not earned.
    for _ in 0..10 {
        s.handle(press_with(false, "POEWindowClass", ANCHOR));
    }

    assert_eq!(s.state(), State::Ready);
    assert_eq!(s.consecutive_unknown(), 0);
}

#[test]
fn a_baseline_read_does_not_require_shift_but_does_require_the_game() {
    let mut s = session(4);
    s.handle(Event::Arm);

    // Nothing is clicked, so Shift is irrelevant — but `Ctrl+C` goes wherever the focus is, and a
    // copy taken out of our own window would be nonsense.
    let elsewhere = s.handle(press_with(false, "Tauri Window", ANCHOR));
    assert!(elsewhere.plan().is_empty());
    assert_eq!(s.state(), State::Sighting);
    assert_eq!(s.anchor(), None, "a refused press captures no Anchor");

    let in_game = s.handle(press_with(false, "POEWindowClass", ANCHOR));
    assert!(!in_game.plan().is_empty());
    assert!(!spends_an_orb(in_game.plan()));
    assert_eq!(s.anchor(), Some(ANCHOR));
}

#[test]
fn a_resync_needs_the_cursor_on_the_anchor_so_it_reads_the_right_item() {
    let mut s = ready_session(4);
    s.handle(press());
    s.handle(rolled_blind());
    assert_eq!(s.state(), State::Resyncing);

    let drifted = s.handle(press_with(
        true,
        "POEWindowClass",
        (ANCHOR.0 + 400, ANCHOR.1),
    ));

    assert!(drifted.plan().is_empty());
    assert_eq!(s.state(), State::Resyncing, "still owed a Verdict");
}

// ── Dropped presses, and suppression ───────────────────────────────────────────────────────────

/// What this does and does not cover.
///
/// It covers the session's own answer to a press arriving in `Rolling`: no plan, counted, never a
/// second orb. In production that branch is close to unreachable — `win32`'s worker is single-threaded
/// and busy inside the plan, so it feeds no press at all while a cycle is in flight and reports the
/// batch afterwards as `PressesDropped`, which is the second half below.
///
/// It does **not** cover which presses `win32` decides to drop, because that lives in the worker loop
/// and cannot run here. The newest press of a mid-cycle batch is *not* dropped: it is served once the
/// cycle ends. Safe, and for the reason the whole crate is shaped this way — it is judged from scratch
/// when it is served, so it still needs a fresh Miss to Click. See `worker_loop` in
/// `crates/win32/src/cycle.rs`.
#[test]
fn presses_arriving_during_a_cycle_are_counted_and_dropped_rather_than_queued() {
    let mut s = ready_session(4);
    s.handle(press());
    assert_eq!(s.state(), State::Rolling);

    let outcome = s.handle(press());
    assert!(
        outcome.plan().is_empty(),
        "never queued — a queued press is a second orb"
    );
    s.handle(Event::PressesDropped { count: 2 });

    assert_eq!(s.presses_dropped(), 3);
    assert_eq!(
        s.rolls(),
        0,
        "the click for this cycle has not reported yet"
    );
}

#[test]
fn the_trigger_key_is_suppressed_in_every_state_but_idle() {
    // Suppression is one relaxed atomic load in the hook callback, so it can only be this coarse.
    // Idle leaves the key alone system-wide; everything else swallows it, including Halted — a Halt
    // is not something the human notices instantly, and their next few reflex presses would
    // otherwise leak the trigger straight into the game.
    let mut s = session(4);
    assert!(!s.suppresses_trigger(), "Idle");

    s.handle(Event::Arm);
    assert!(s.suppresses_trigger(), "Sighting");
    press_and_read(&mut s, &missing_capture());
    assert!(s.suppresses_trigger(), "Ready");
    s.handle(press());
    assert_eq!(s.state(), State::Rolling);
    assert!(s.suppresses_trigger(), "Rolling");
    s.handle(rolled_blind());
    assert_eq!(s.state(), State::Resyncing);
    assert!(s.suppresses_trigger(), "Resyncing");

    press_and_read(&mut s, &t4_capture());
    assert_eq!(s.state(), State::Latched);
    assert!(s.suppresses_trigger(), "Latched");

    s.handle(Event::Acknowledge);
    press_and_read(&mut s, &wrong_item());
    assert_eq!(s.state(), State::Halted);
    assert!(s.suppresses_trigger(), "Halted");

    s.handle(Event::Disarm);
    assert!(!s.suppresses_trigger(), "Idle again");
}

// ── Everything reaches the log ─────────────────────────────────────────────────────────────────

#[test]
fn every_outcome_that_matters_says_something_for_the_log() {
    // The gaming PC has no dev environment and the updater force-exits the app, so a finding that
    // only ever rendered is a finding lost ([#11](https://github.com/Furizaa/poe-graft/issues/11)).
    let mut s = session(4);
    for event in [
        Event::Arm,
        press(),
        read(&missing_capture()),
        press(),
        rolled_blind(),
        press_with(false, "POEWindowClass", ANCHOR),
        press(),
        read(&t4_capture()),
        Event::Acknowledge,
        Event::Disarm,
    ] {
        let before = s.state();
        let outcome = s.handle(event);
        assert!(
            !outcome.log().is_empty(),
            "in {before:?}, nothing was written down"
        );
        for line in outcome.log() {
            assert!(!line.trim().is_empty());
        }
    }
}

#[test]
fn the_message_explains_why_the_session_halted() {
    let mut s = ready_session(4);
    press_and_fail(&mut s);
    press_and_fail(&mut s);
    press_and_fail(&mut s);

    // The badge names the state; after a Halt the human needs to read *why*, and that copy is core's
    // rather than the frontend's so the log and the window cannot disagree.
    assert_eq!(s.state(), State::Halted);
    let message = s.message();
    assert!(message.contains('3'), "{message:?}");
    assert!(message.contains("Unknown"), "{message:?}");
    assert!(message.contains("Re-arm"), "{message:?}");
}

// ── Replaying the spike ────────────────────────────────────────────────────────────────────────

#[test]
fn the_whole_spike_manifest_replays_without_ever_rolling_on_an_unknown() {
    // All 81 roll records from the three armed sessions of
    // [#17](https://github.com/Furizaa/poe-graft/issues/17), in order, with the spike's own verdict
    // line: 47 carry Item Text, 34 do not, and 5 of the 47 are `IDENTICAL to the previous roll`. The
    // sessions ran at a 40 ms settle delay, since retired for 130 ms, so the failure *rate* in here
    // measures a delay we no longer use — the *sequence* is what this test wants.
    let records = manifest("spike-17");
    assert_eq!(records.len(), 81);

    let mut s = session(1);
    let mut orbs = 0;
    let mut unknowns = 0;
    let mut served = 0;
    let mut halts = 0;

    s.handle(Event::Arm);
    for record in &records {
        if s.state() == State::Halted {
            // A run of three Unknowns, which at 40 ms is the delay's fault and not the item's.
            halts += 1;
            s.handle(Event::Arm);
        }

        let plan = s.handle(press()).plan().to_vec();
        assert!(
            !plan.is_empty(),
            "every press here satisfies its preconditions"
        );
        served += 1;
        let rolled = spends_an_orb(&plan);
        if rolled {
            orbs += 1;
        }

        let read = match &record.capture {
            Some(file) => ReadOutcome::Text(common::capture(&format!("spike-17/{file}"))),
            None => ReadOutcome::NothingCopied,
        };
        s.handle(Event::CycleFinished(CycleReport { rolled, read }));
        if s.last_verdict() == Some(Verdict::Unknown) {
            unknowns += 1;
        }
    }

    assert_eq!(served, 81, "every record became a press the cycle served");
    assert!(
        halts > 0,
        "the 40 ms sessions must have tripped the Unknown limit"
    );
    assert!(
        orbs < served,
        "{orbs} orbs for {served} presses — the difference is the baseline Reads and the Resyncs"
    );
    assert!(
        unknowns >= 34,
        "at least the 34 textless records are Unknown, got {unknowns}"
    );
    // None of the 41 captures carries the target at T1, so the replay never Latches. That is the
    // honest outcome: the spike never rolled a T1 either.
    assert_ne!(s.state(), State::Latched);
}

#[test]
fn the_five_identical_reads_in_the_manifest_are_the_stale_read_case() {
    let records = manifest("spike-17");
    let flagged: Vec<usize> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.verdict.contains("IDENTICAL"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(flagged.len(), 5);

    for &i in &flagged {
        let capture = records[i]
            .capture
            .as_ref()
            .expect("a flagged record has text");
        // The spike flagged it by comparing against the previous roll's text, and the manifest keeps
        // that: the same file, twice in a row.
        assert_eq!(records[i - 1].capture.as_ref(), Some(capture));

        // The spike counted it and rolled on regardless. Every one of those presses was a ~1-in-278
        // chance of having just destroyed the Hit it made.
        let text = common::capture(&format!("spike-17/{capture}"));
        let mut s = session(1);
        s.handle(Event::Arm);
        press_and_read(&mut s, &text);
        assert_eq!(s.state(), State::Ready, "the baseline Read is a Miss at T1");

        press_and_read(&mut s, &text);
        assert_eq!(s.state(), State::Resyncing);
        assert_eq!(s.last_verdict(), Some(Verdict::Unknown));
    }
}

// ── The invariant ──────────────────────────────────────────────────────────────────────────────

#[test]
fn no_reachable_path_clicks_without_a_fresh_miss() {
    // **The app never spends an Alteration without a fresh Miss for the item in front of it.**
    //
    // Asserted over the *whole reachable state space* rather than a sample of event sequences: a
    // breadth-first walk from `Idle`, deduplicated on `CraftSession::fingerprint`, which describes
    // everything the next decision depends on. Every emitted `Click` must come from a session that
    // was in `Ready` holding a Miss, and `Ready` must be unreachable without one. Together those two
    // are the invariant, and a counterexample prints the events that produced it.
    let alphabet: Vec<Event> = vec![
        Event::Arm,
        Event::Disarm,
        Event::Acknowledge,
        Event::PressesDropped { count: 1 },
        press(),
        press_with(false, "POEWindowClass", ANCHOR),
        press_with(true, "Tauri Window", ANCHOR),
        press_with(true, "POEWindowClass", (ANCHOR.0 + 500, ANCHOR.1)),
        rolled(&missing_capture()),
        rolled(&another_miss()),
        rolled(&t4_capture()),
        rolled(&wrong_item()),
        rolled_blind(),
        read(&missing_capture()),
        read(&t4_capture()),
        Event::CycleFinished(CycleReport {
            rolled: false,
            read: ReadOutcome::NothingReadable,
        }),
    ];

    let root = session(4);
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(root.fingerprint());
    let mut queue: VecDeque<(CraftSession, Vec<String>)> = VecDeque::new();
    queue.push_back((root, Vec::new()));

    let mut nodes = 0usize;
    let mut clicks = 0usize;

    while let Some((node, path)) = queue.pop_front() {
        nodes += 1;
        for event in &alphabet {
            let mut next = node.clone();
            let before = (next.state(), next.last_verdict());
            let outcome = next.handle(event.clone());

            let mut trail = path.clone();
            trail.push(describe(event));

            if spends_an_orb(outcome.plan()) {
                assert_eq!(
                    before,
                    (State::Ready, Some(Verdict::Miss)),
                    "an Alteration was spent without a fresh Miss behind it, after {trail:?}"
                );
                clicks += 1;
            }

            if next.state() == State::Ready {
                assert_eq!(
                    next.last_verdict(),
                    Some(Verdict::Miss),
                    "Ready is the state that authorises a Click, so nothing but a Miss may reach \
                     it — after {trail:?}"
                );
            }

            if seen.insert(next.fingerprint()) {
                queue.push_back((next, trail));
            }
        }
    }

    // 56 distinct states and 4 of them authorise a Click, as of this alphabet. The bounds are loose
    // on purpose — they are here to catch a change that makes the walk stop exploring, not to pin a
    // number that will move whenever a state or an event is added.
    assert!(
        nodes > 20,
        "only {nodes} distinct states — the walk is not exploring"
    );
    assert!(clicks > 0, "the walk never clicked, so it proved nothing");
}

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────

struct Record {
    verdict: String,
    capture: Option<String>,
}

/// The ordered roll records for a capture set.
fn manifest(set: &str) -> Vec<Record> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/captures")
        .join(set)
        .join("manifest.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let raw: serde_json::Value = serde_json::from_str(&json).expect("the manifest is JSON");

    raw.as_array()
        .expect("the manifest is an array of records")
        .iter()
        .map(|r| Record {
            verdict: r["verdict"].as_str().expect("a verdict line").to_string(),
            capture: r["capture"].as_str().map(str::to_string),
        })
        .collect()
}

/// Short enough to read in a failure message, specific enough to reproduce it.
fn describe(event: &Event) -> String {
    match event {
        Event::Arm => "Arm".into(),
        Event::Disarm => "Disarm".into(),
        Event::Acknowledge => "Acknowledge".into(),
        Event::PressesDropped { count } => format!("PressesDropped({count})"),
        Event::Press(p) => format!(
            "Press(shift {}, {}, {:?})",
            p.shift_down, p.foreground_class, p.cursor
        ),
        Event::CycleFinished(r) => format!(
            "CycleFinished(rolled {}, {})",
            r.rolled,
            match &r.read {
                ReadOutcome::Text(t) => format!("{} chars of text", t.len()),
                other => format!("{other:?}"),
            }
        ),
    }
}
