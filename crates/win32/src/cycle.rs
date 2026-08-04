//! The executor: the Windows half of the roll cycle.
//!
//! `poe-graft-core` owns the decisions; this owns the hands. A `WH_KEYBOARD_LL` hook counts physical
//! Trigger Presses, a worker thread reads the three facts a press needs, feeds them to the
//! [`CraftSession`], and executes whatever plan comes back — poison, click, settle, copy, read — then
//! reports what happened. Nothing here decides anything.
//!
//! Every mechanism in this file was proven on the gaming PC by the throwaway spike for
//! [#17](https://github.com/Furizaa/poe-graft/issues/17), which this replaces. The comments that read
//! like scars are scars.
//!
//! # The TOS rule this obeys
//!
//! Exactly **one** injected Shift+left-click and **one** `Ctrl+C` per Trigger Press the human
//! physically makes, and nothing otherwise. No timer, no repeat without a fresh press, nothing done
//! in reaction to what was read. Four mechanisms enforce that rather than merely intending it:
//!
//! * `LLKHF_INJECTED` presses are ignored, so the app can never react to its own input.
//! * Auto-repeat is filtered by tracking the key's own up/down state, so resting a finger on the
//!   trigger is one press rather than one press per keyboard repeat interval.
//! * A cycle already in flight makes the next press a counted no-op rather than a queued action.
//! * On a Hit the cycle Latches, and a Latched session emits no commands at all — the app *refuses*
//!   the next press instead of acting on the Read.
//!
//! # The hook callback's budget
//!
//! Every keyboard event on the desktop waits inside [`keyboard_hook`]. The budget is 300 ms
//! (`LowLevelHooksTimeout`), and on the 11th overrun Windows **silently uninstalls the hook** with no
//! way for the app to notice — it fails *open*. So the callback does nothing but relaxed atomic
//! loads, integer comparisons and one `fetch_add`. No allocation, no locks, no I/O, no logging, and
//! above all no injection: that happens on the worker thread. In particular it must never call
//! `GetForegroundWindow`, which is why suppression is scoped to "is a Craft Session armed" — one
//! relaxed load — rather than to which window has focus.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use poe_graft_core::{
    Command, CraftSession, CycleReport, Event, Feedback, PlatformError, Press, ReadOutcome, Target,
    Verdict,
};
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Accessibility::{
    FILTERKEYS, SKF_AVAILABLE, SKF_STICKYKEYSON, STICKYKEYS, TOGGLEKEYS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, VK_C, VK_CONTROL,
    VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetCursorPos, GetForegroundWindow, GetMessageW, GetWindowTextW,
    PostThreadMessageW, SetWindowsHookExW, SystemParametersInfoW, UnhookWindowsHookEx,
    FKF_FILTERKEYSON, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, SPI_GETFILTERKEYS,
    SPI_GETSTICKYKEYS, SPI_GETTOGGLEKEYS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, TKF_TOGGLEKEYSON,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

/// `[` — `VK_OEM_4`. The default Trigger Key, and the only one with on-device evidence behind it:
/// 60 rolls through the spike with suppression on, and the game never saw it. F13–F24 would be the
/// conventional choice and are out, because this keyboard has no F-row.
pub const DEFAULT_TRIGGER_VK: u32 = 0xDB;

// ---------------------------------------------------------------------------------------------
// Where findings go
// ---------------------------------------------------------------------------------------------

/// Somewhere to put a line of text. `src-tauri` supplies one backed by the app's journal.
pub type LogSink = Box<dyn Fn(&str) + Send + Sync>;

/// The log sink, installed once by `src-tauri`.
///
/// Everything the app learns has to reach the **file**, not just the window: the updater force-exits
/// the app on Windows, and a crashing hook takes the window with it. A finding that only ever
/// rendered is a finding lost.
static SINK: OnceLock<LogSink> = OnceLock::new();

/// Point the cycle's logging at the app's journal. Later calls are ignored.
pub fn set_log_sink(sink: LogSink) {
    let _ = SINK.set(sink);
}

fn log(line: &str) {
    if let Some(sink) = SINK.get() {
        sink(line);
    }
}

// ---------------------------------------------------------------------------------------------
// State the hook callback touches. Atomics only — see the module docs.
// ---------------------------------------------------------------------------------------------

/// Virtual-key code of the Trigger Key. `0` disables the trigger entirely.
static TRIGGER_VK: AtomicU32 = AtomicU32::new(DEFAULT_TRIGGER_VK);
/// Is a Craft Session armed?
///
/// This is *also* the suppression flag, and deliberately the same bit: ADR 0002 scopes suppression to
/// "armed, and only then", so `Idle` leaves `[` alone system-wide and typing it anywhere else on the
/// desktop keeps working.
static ARMED: AtomicBool = AtomicBool::new(false);
/// The Trigger Key's own up/down state, so auto-repeat is not mistaken for a new press.
static TRIGGER_DOWN: AtomicBool = AtomicBool::new(false);
/// Monotonic count of fresh physical Trigger Presses seen while armed. The worker compares this
/// against the number it has served, which is how dropped presses become a recorded number rather
/// than a silent gap.
static PRESS_SEQ: AtomicU32 = AtomicU32::new(0);
/// How many physical key-downs the callback has observed since the hook went in — a **count only**,
/// never which keys.
///
/// It exists because `SetWindowsHookExW` returning a valid handle proves only that Windows accepted
/// the hook, not that it is delivering anything: a hook that installs cleanly and then hears nothing
/// looks identical, from the app, to a panel failing to render what it heard. This number tells those
/// two apart in one glance, and it is the only reason
/// [#18](https://github.com/Furizaa/poe-graft/issues/18) was ever visible.
static KEYS_SEEN: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------------------------
// State only the worker and the commands touch
// ---------------------------------------------------------------------------------------------

/// Set while the worker thread should keep running.
static WORKER_RUN: AtomicBool = AtomicBool::new(false);

/// The Craft Session. `src-tauri` locks it to read a status; the worker locks it to feed events.
///
/// Deliberately **not** held across command execution: a plan takes 130–280 ms to run, and holding
/// the lock that long would stall the status poll and the human's Acknowledge click. The cost is that
/// the human can disarm mid-cycle, which is exactly what `CraftSession`'s discarded-report guard is
/// for.
static SESSION: Mutex<Option<CraftSession>> = Mutex::new(None);

/// The installed hook's threads, so it can be taken down again.
static HOOK: Mutex<Option<Installed>> = Mutex::new(None);

/// How long the last cycle took, for the window.
static LAST_TIMING: Mutex<Option<Timing>> = Mutex::new(None);

/// Feedback counters. The window polls them and plays one sound per increment — simpler than a queue,
/// and a counter cannot lose or duplicate an event the way a drained queue can.
static HITS: AtomicU32 = AtomicU32::new(0);
static HALTS: AtomicU32 = AtomicU32::new(0);
static BLIPS: AtomicU32 = AtomicU32::new(0);

struct Installed {
    hook_thread_id: u32,
    hook_thread: JoinHandle<()>,
    worker: JoinHandle<()>,
}

/// What the last cycle cost, in milliseconds.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// `Ctrl+C` → the clipboard changed. Our own code does this in 1–8 ms.
    pub copy_ms: u32,
    /// The whole plan, click to text in hand.
    pub cycle_ms: u32,
}

/// Accessibility settings that silently change how held modifiers behave.
#[derive(Debug, Clone, Copy)]
pub struct Accessibility {
    /// Sticky Keys is **on**. This is the one that silently breaks Apply Mode.
    pub sticky_keys_on: bool,
    /// Sticky Keys is available to be switched on — e.g. by the five-taps-on-Shift shortcut, which is
    /// exactly the gesture a Shift-heavy crafting session might trip by accident.
    pub sticky_keys_available: bool,
    /// Filter Keys is on: it drops or delays repeated keystrokes.
    pub filter_keys_on: bool,
    /// Toggle Keys is on: harmless here, but it confirms the read is working.
    pub toggle_keys_on: bool,
}

/// Everything the window shows. Plain structs — the serde DTO lives in `src-tauri`, which is the seam
/// doing its job.
#[derive(Debug, Clone)]
pub struct CycleStatus {
    /// Is the hook installed and the worker alive?
    pub running: bool,
    /// The state badge.
    pub state: &'static str,
    /// The most recent thing worth saying, in core's words.
    pub message: String,
    /// The Target Mod's Mod Group.
    pub target_group: String,
    /// The worst tier that still counts as a Hit.
    pub tier_threshold: u8,
    /// Alterations spent this Craft Session.
    pub rolls: u32,
    /// Where the jewel is, once the baseline Read captured it.
    pub anchor: Option<(i32, i32)>,
    /// Presses dropped because a cycle was in flight.
    pub presses_dropped: u32,
    /// Consecutive `Unknown` Verdicts, and how many end the session.
    pub consecutive_unknown: u32,
    /// The limit the count above is racing.
    pub unknown_limit: u32,
    /// What the last completed Read established.
    pub last_verdict: Option<&'static str>,
    /// The tier its numbers implied.
    pub last_tier: Option<u8>,
    /// Why the session Halted, if it did.
    pub halt_reason: Option<String>,
    /// Is the Trigger Key being swallowed right now?
    pub suppress: bool,
    /// The Trigger Key.
    pub trigger_vk: u32,
    /// The Trigger Key in words.
    pub trigger_name: String,
    /// Physical key-downs the hook has seen. Zero while installed means the hook is deaf.
    pub keys_seen: u32,
    /// Physical Trigger Presses seen while armed.
    pub presses: u32,
    /// Is Shift held right now?
    pub shift_down: bool,
    /// The foreground window, for the human to compare against the Refusal message.
    pub foreground: String,
    /// Monotonic count of Hits, for the sound.
    pub hits: u32,
    /// Monotonic count of Halts, for the sound.
    pub halts: u32,
    /// Monotonic count of blips, for the sound.
    pub blips: u32,
    /// What the last cycle cost.
    pub timing: Option<Timing>,
}

// ---------------------------------------------------------------------------------------------
// The hook callback
// ---------------------------------------------------------------------------------------------

/// # Safety
///
/// Called by Windows with `lparam` pointing at a live [`KBDLLHOOKSTRUCT`] for the duration of the
/// call. Nothing here outlives it.
unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Anything other than HC_ACTION must be passed straight on, undecoded.
    if code != HC_ACTION as i32 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    // SAFETY: for HC_ACTION on a WH_KEYBOARD_LL hook Windows guarantees `lparam` is a valid
    // `KBDLLHOOKSTRUCT` pointer, valid for reads for the length of this call. We only read.
    let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };

    let message = wparam.0 as u32;
    let is_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    // Our own `SendInput` traffic comes back through this hook. Ignoring it is what makes "one action
    // per *physical* press" a property of the code rather than a hope.
    let injected = event.flags.contains(LLKHF_INJECTED);

    if is_down && !injected {
        KEYS_SEEN.fetch_add(1, Ordering::Relaxed);
    }

    let trigger = TRIGGER_VK.load(Ordering::Relaxed);
    if trigger == 0 || event.vkCode != trigger || injected {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    if is_down {
        // `swap` returning false means the key was up, so this is a fresh press rather than the
        // auto-repeat stream Windows sends while a key is held. Without this filter, resting a finger
        // on the trigger is one action per repeat interval, which is a TOS violation rather than a UX
        // wrinkle.
        if !TRIGGER_DOWN.swap(true, Ordering::Relaxed) && ARMED.load(Ordering::Relaxed) {
            PRESS_SEQ.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        TRIGGER_DOWN.store(false, Ordering::Relaxed);
    }

    if ARMED.load(Ordering::Relaxed) {
        // Non-zero: the event dies here and Path of Exile never sees it. Measured on device — unlike
        // the mouse, keyboard suppression does reach the client.
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

// ---------------------------------------------------------------------------------------------
// Start / stop
// ---------------------------------------------------------------------------------------------

/// Take ownership of a Craft Session, install the keyboard hook, and start the worker.
///
/// The hook is installed *on the thread that pumps messages for it*, which is a Win32 requirement
/// rather than a style choice: a low-level hook is serviced by its owning thread's message loop, so a
/// hook installed on a thread that never calls `GetMessageW` silently never fires.
pub fn start(session: CraftSession) -> Result<(), PlatformError> {
    let mut slot = lock(&HOOK);
    if slot.is_some() {
        return Ok(());
    }

    // The session goes in before the hook does, so a press can never arrive with nothing to feed.
    ARMED.store(session.suppresses_trigger(), Ordering::Release);
    *lock(&SESSION) = Some(session);

    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();

    let hook_thread = std::thread::Builder::new()
        .name("poe-graft-hook".into())
        .spawn(move || {
            // SAFETY: `keyboard_hook` is a `'static` function in this process. `None` for the module
            // handle is correct for a low-level hook whose procedure lives in the calling process;
            // thread id 0 makes it global.
            let hook =
                match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0) } {
                    Ok(hook) => hook,
                    Err(err) => {
                        let _ = ready_tx.send(Err(err.message()));
                        return;
                    }
                };

            // SAFETY: a plain read of the calling thread's own id.
            let thread_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
            if ready_tx.send(Ok(thread_id)).is_err() {
                // Nobody is listening any more, so unwind rather than leaving a hook installed.
                let _ = unsafe { UnhookWindowsHookEx(hook) };
                return;
            }

            // Servicing loop. A low-level hook is dispatched by its owning thread's message pump, so
            // this loop existing is what makes the hook fire at all. There is no window on this
            // thread, so there is nothing to translate or dispatch.
            //
            // `stop` ends it with a posted WM_QUIT, for which `GetMessageW` returns 0. It returns -1
            // on error, and treating that as "keep going" would spin a core forever, so anything not
            // strictly positive stops the loop.
            let mut message = MSG::default();
            loop {
                // SAFETY: `message` is ours and outlives each call.
                let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
                if result.0 <= 0 {
                    break;
                }
            }

            // SAFETY: `hook` was returned by `SetWindowsHookExW` on this thread and has not been
            // unhooked yet.
            if let Err(err) = unsafe { UnhookWindowsHookEx(hook) } {
                log(&format!("cycle: unhook failed: {}", err.message()));
            }
        })
        .map_err(|err| PlatformError::Os {
            capability: "spawn hook thread",
            detail: err.to_string(),
        })?;

    let hook_thread_id = match ready_rx.recv() {
        Ok(Ok(id)) => id,
        Ok(Err(detail)) => {
            let _ = hook_thread.join();
            return Err(PlatformError::Os {
                capability: "SetWindowsHookExW(WH_KEYBOARD_LL)",
                detail,
            });
        }
        Err(_) => {
            let _ = hook_thread.join();
            return Err(PlatformError::Os {
                capability: "SetWindowsHookExW(WH_KEYBOARD_LL)",
                detail: "the hook thread died before reporting".into(),
            });
        }
    };

    KEYS_SEEN.store(0, Ordering::Relaxed);

    WORKER_RUN.store(true, Ordering::Release);
    let worker = std::thread::Builder::new()
        .name("poe-graft-cycle".into())
        .spawn(worker_loop)
        .map_err(|err| PlatformError::Os {
            capability: "spawn worker thread",
            detail: err.to_string(),
        })?;

    *slot = Some(Installed {
        hook_thread_id,
        hook_thread,
        worker,
    });

    log(&format!(
        "cycle: WH_KEYBOARD_LL installed · Trigger Key {}",
        describe_vk(TRIGGER_VK.load(Ordering::Relaxed))
    ));
    log_accessibility();
    Ok(())
}

/// Take the hook down and stop the worker. Also disarms — an uninstalled hook that still believed
/// itself armed would be the worst kind of confusing.
pub fn stop() -> Result<(), PlatformError> {
    let installed = lock(&HOOK).take();
    let Some(installed) = installed else {
        return Ok(());
    };

    feed(Event::Disarm);
    ARMED.store(false, Ordering::Release);
    WORKER_RUN.store(false, Ordering::Release);

    // SAFETY: posting a message to a thread id is safe; a dead thread just makes it fail.
    if let Err(err) =
        unsafe { PostThreadMessageW(installed.hook_thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) }
    {
        log(&format!("cycle: could not post WM_QUIT: {}", err.message()));
    }

    let _ = installed.hook_thread.join();
    let _ = installed.worker.join();
    log("cycle: WH_KEYBOARD_LL removed");
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------------------------

/// Arm or disarm a Craft Session.
pub fn arm(on: bool) -> Result<(), PlatformError> {
    if lock(&HOOK).is_none() {
        return Err(PlatformError::Os {
            capability: "arm",
            detail: "the keyboard hook is not installed".into(),
        });
    }
    feed(if on { Event::Arm } else { Event::Disarm });
    Ok(())
}

/// Acknowledge a Latched Hit. A mouse click in our own window, and the only thing that releases it.
pub fn acknowledge() {
    feed(Event::Acknowledge);
}

/// Choose the Target Mod. Returns false if a Craft Session is under way, which is the one time it
/// must not change — the session is holding a Verdict about the old target.
pub fn set_target(target: Target) -> bool {
    let mut guard = lock(&SESSION);
    let Some(session) = guard.as_mut() else {
        return false;
    };
    let accepted = session.set_target(target);
    let described = format!(
        "{} at Tier {} or better",
        session.target().group_id(),
        session.target().tier_threshold()
    );
    drop(guard);
    if accepted {
        log(&format!("cycle: Target Mod set to {described}"));
    } else {
        log("cycle: refused to change the Target Mod — a Craft Session is armed");
    }
    accepted
}

/// Choose the Trigger Key. `0` disables the trigger entirely.
///
/// Typed as a code rather than learned by pressing, because
/// [#18](https://github.com/Furizaa/poe-graft/issues/18) makes the hook deaf while our own window has
/// focus — which is exactly when the human would be trying to teach it a key.
pub fn set_trigger(vk: u32) {
    TRIGGER_VK.store(vk, Ordering::Relaxed);
    TRIGGER_DOWN.store(false, Ordering::Relaxed);
    log(&format!("cycle: Trigger Key set to {}", describe_vk(vk)));
}

/// Write one of the human's own observations into the machine's log, in order, timestamped.
pub fn note(line: &str) {
    log(&format!("note — {line}"));
}

/// Everything the window shows.
pub fn status() -> CycleStatus {
    let trigger_vk = TRIGGER_VK.load(Ordering::Relaxed);
    let (_, foreground) = foreground();

    // Every lock taken and released before the next one, and `SESSION` taken last. `start` holds
    // `HOOK` while it installs `SESSION`, so acquiring them the other way round here — which is what
    // reading `running` inside the struct literal did — is a lock-order inversion, and the window
    // polls this eight times a second.
    let running = lock(&HOOK).is_some();
    let timing = *lock(&LAST_TIMING);

    let guard = lock(&SESSION);
    let session = guard.as_ref();

    CycleStatus {
        running,
        state: session.map(|s| s.state().label()).unwrap_or("Idle"),
        message: session
            .map(|s| s.message().to_string())
            .unwrap_or_else(|| "The roll cycle is not running.".into()),
        target_group: session
            .map(|s| s.target().group_id().to_string())
            .unwrap_or_default(),
        tier_threshold: session.map(|s| s.target().tier_threshold()).unwrap_or(0),
        rolls: session.map(|s| s.rolls()).unwrap_or(0),
        anchor: session.and_then(|s| s.anchor()),
        presses_dropped: session.map(|s| s.presses_dropped()).unwrap_or(0),
        consecutive_unknown: session.map(|s| s.consecutive_unknown()).unwrap_or(0),
        unknown_limit: session.map(|s| s.config().unknown_limit).unwrap_or(0),
        last_verdict: session.and_then(|s| s.last_verdict()).map(|v| match v {
            Verdict::Hit => "Hit",
            Verdict::Miss => "Miss",
            Verdict::Unknown => "Unknown",
        }),
        last_tier: session.and_then(|s| s.last_tier()),
        halt_reason: session.and_then(|s| s.halt_reason().map(|r| r.to_string())),
        suppress: ARMED.load(Ordering::Acquire),
        trigger_vk,
        trigger_name: describe_vk(trigger_vk),
        keys_seen: KEYS_SEEN.load(Ordering::Relaxed),
        presses: PRESS_SEQ.load(Ordering::Relaxed),
        shift_down: key_down(VK_SHIFT.0 as i32),
        foreground,
        hits: HITS.load(Ordering::Relaxed),
        halts: HALTS.load(Ordering::Relaxed),
        blips: BLIPS.load(Ordering::Relaxed),
        timing,
    }
}

// ---------------------------------------------------------------------------------------------
// The worker: one cycle per physical press
// ---------------------------------------------------------------------------------------------

fn worker_loop() {
    let mut handled = PRESS_SEQ.load(Ordering::Relaxed);

    while WORKER_RUN.load(Ordering::Acquire) {
        let seq = PRESS_SEQ.load(Ordering::Relaxed);
        if seq == handled {
            // Nothing to serve. Sleep granularity is coarse on Windows and it does not matter — this
            // adds a millisecond or two to a human-paced press, and every measurement that matters is
            // taken with `Instant` around a busy wait instead.
            //
            // While no Craft Session is armed the hook does not count presses at all, so `PRESS_SEQ`
            // cannot move and there is nothing to be responsive to. Waking a thousand times a second
            // to discover that, next to a running game, would be rude for no reason.
            std::thread::sleep(Duration::from_millis(if ARMED.load(Ordering::Relaxed) {
                1
            } else {
                20
            }));
            continue;
        }

        // Everything between `handled` and `seq` beyond the one about to be served arrived while the
        // previous cycle was in flight, and was deliberately dropped rather than queued.
        let dropped = seq - handled - 1;
        handled = seq;
        serve(dropped);
    }
}

fn serve(dropped: u32) {
    if dropped > 0 {
        feed(Event::PressesDropped { count: dropped });
    }

    // The three facts a press is judged on, read here rather than in the hook callback — the callback
    // may not call `GetForegroundWindow`, and it may not allocate the strings this needs.
    let cursor = match cursor_pos() {
        Ok(cursor) => cursor,
        Err(err) => {
            log(&format!(
                "cycle: Trigger Press dropped — could not read the cursor: {err}"
            ));
            return;
        }
    };
    let (foreground_class, foreground) = foreground();

    let plan = feed(Event::Press(Press {
        shift_down: key_down(VK_SHIFT.0 as i32),
        foreground_class,
        cursor,
    }));

    if plan.is_empty() {
        // Refused, dropped, Latched or Halted. Core has already said why; the foreground is worth
        // adding, because "not Path of Exile" is only useful alongside what *was* in front.
        log(&format!("cycle: no commands · foreground {foreground}"));
        return;
    }

    let report = execute(&plan);
    feed(Event::CycleFinished(report));
}

/// Hand an event to the Craft Session, then act on everything it asked for.
///
/// The lock is held only for the transition itself. Logging and the feedback counters happen outside
/// it, so a slow file write can never stall the status poll.
fn feed(event: Event) -> Vec<Command> {
    let mut guard = lock(&SESSION);
    let Some(session) = guard.as_mut() else {
        return Vec::new();
    };

    let outcome = session.handle(event);
    // Suppression follows the state, always, and is published before the lock is dropped so a press
    // arriving right now sees the new value.
    ARMED.store(session.suppresses_trigger(), Ordering::Release);

    let plan = outcome.plan().to_vec();
    let feedback = outcome.feedback();
    let lines = outcome.log().to_vec();
    drop(guard);

    for line in lines {
        log(&line);
    }
    match feedback {
        Some(Feedback::Hit) => HITS.fetch_add(1, Ordering::Relaxed),
        Some(Feedback::Halt) => HALTS.fetch_add(1, Ordering::Relaxed),
        Some(Feedback::Blip) => BLIPS.fetch_add(1, Ordering::Relaxed),
        None => 0,
    };

    plan
}

/// Run a plan, in order, and report what happened.
///
/// Nothing here is conditional on what comes back: the plan was fixed before the first command ran,
/// which is what keeps the app out of the "acts in reaction to a read" category the policy prohibits.
fn execute(plan: &[Command]) -> CycleReport {
    let started = Instant::now();
    let mut rolled = false;
    let mut sentinel = String::new();
    let mut poisoned_seq = 0u32;
    let mut copy_started: Option<Instant> = None;
    let mut copy_ms = 0u32;
    // A plan with no `AwaitRead` would be a core bug rather than an OS failure, and this is what it
    // would look like in the log.
    let mut read = ReadOutcome::Failed("the plan asked for no Read".into());

    for command in plan {
        match command {
            Command::Poison { sentinel: text } => {
                // Poison first. Content comparison cannot prove freshness — two identical rolls give
                // identical bytes — so the clipboard has to hold something that could only have come
                // from us before the copy is asked for.
                if let Err(err) =
                    clipboard_win::set_clipboard(clipboard_win::formats::Unicode, text)
                {
                    return CycleReport {
                        rolled,
                        read: ReadOutcome::Failed(format!("could not poison the clipboard: {err}")),
                    };
                }
                sentinel = text.clone();
                poisoned_seq = clipboard_win::seq_num().map(|n| n.get()).unwrap_or(0);
            }
            Command::Click => {
                // The one game action. No move flag and no coordinates, so this lands wherever the
                // cursor already is and the cursor never moves.
                if let Err(err) = click_left() {
                    return CycleReport {
                        rolled,
                        read: ReadOutcome::Failed(format!("the injected click failed: {err}")),
                    };
                }
                // Reported the instant the click lands. An orb is spent whether or not the Read
                // afterwards succeeds, times out or fails outright.
                rolled = true;
            }
            Command::Settle { ms } => {
                // `sleep`, not a busy wait: this is the longest part of the cycle, it needs no
                // sub-millisecond accuracy, and spinning here would burn most of a core next to a
                // running game for no benefit.
                std::thread::sleep(Duration::from_millis(*ms as u64));
            }
            Command::SendCopy => {
                copy_started = Some(Instant::now());
                if let Err(err) = send_copy() {
                    return CycleReport {
                        rolled,
                        read: ReadOutcome::Failed(format!("the injected Ctrl+C failed: {err}")),
                    };
                }
            }
            Command::AwaitRead {
                timeout_ms,
                settle_ms,
            } => {
                let from = copy_started.unwrap_or_else(Instant::now);
                read = await_read(&sentinel, poisoned_seq, *timeout_ms, *settle_ms, from);
                copy_ms = from.elapsed().as_millis() as u32;
            }
        }
    }

    *lock(&LAST_TIMING) = Some(Timing {
        copy_ms,
        cycle_ms: started.elapsed().as_millis() as u32,
    });

    CycleReport { rolled, read }
}

/// Wait for the game to replace the Sentinel, then read what it left.
fn await_read(
    sentinel: &str,
    poisoned_seq: u32,
    timeout_ms: u32,
    settle_ms: u32,
    copy_started: Instant,
) -> ReadOutcome {
    // Poll the sequence number rather than the contents: `GetClipboardSequenceNumber` needs no open
    // clipboard handle, so this never contends with the game for the clipboard lock. Busy waiting,
    // because the answer is expected in single-digit milliseconds and `sleep` on Windows rounds to
    // the scheduler tick, which would swamp the measurement.
    let timeout = Duration::from_millis(timeout_ms as u64);
    let mut moved = false;
    while copy_started.elapsed() < timeout {
        if clipboard_win::seq_num().map(|n| n.get()).unwrap_or(0) != poisoned_seq {
            moved = true;
            break;
        }
        std::hint::spin_loop();
    }

    if !moved {
        // Which of the two possible faults is this? Either the game never copied — our Sentinel is
        // still sitting there untouched — or it did and the sequence-number comparison is wrong, in
        // which case the Item Text is right there. One read settles it, and without it a timeout is
        // just a shrug.
        log(&format!(
            "cycle: the clipboard sequence number never moved in {timeout_ms}ms; it now holds {}",
            classify_clipboard(sentinel)
        ));
        return ReadOutcome::NothingCopied;
    }

    // The sequence number moved, but that does **not** mean the text is there yet. `EmptyClipboard`
    // bumps the counter, and Path of Exile calls it *before* `SetClipboardData` — so reading the
    // instant the number changes can legitimately find an empty clipboard. Roll 14 of the spike's
    // first on-device session was exactly this: `OSError(1168): Element not found`, 2 ms after the
    // bump.
    //
    // Retrying injects nothing and presses nothing: it is purely a read, so it stays inside the
    // one-click-one-copy rule.
    let settle_deadline = Instant::now() + Duration::from_millis(settle_ms as u64);
    let mut last_error = String::new();
    loop {
        match clipboard_win::get_clipboard::<String, _>(clipboard_win::formats::Unicode) {
            Ok(text) if !text.is_empty() && text != sentinel => return ReadOutcome::Text(text),
            Ok(_) => {}
            Err(err) => last_error = err.to_string(),
        }
        if Instant::now() >= settle_deadline {
            break;
        }
        std::hint::spin_loop();
    }

    log(&format!(
        "cycle: the sequence number moved but nothing readable arrived within {settle_ms}ms; the \
         clipboard holds {}{}",
        classify_clipboard(sentinel),
        if last_error.is_empty() {
            String::new()
        } else {
            format!(", last error: {last_error}")
        }
    ));
    ReadOutcome::NothingReadable
}

/// What is actually in the clipboard right now, in words. The decisive diagnostic when a Read goes
/// wrong: "still our Sentinel" and "Item Text" mean completely different faults.
fn classify_clipboard(sentinel: &str) -> String {
    match clipboard_win::get_clipboard::<String, _>(clipboard_win::formats::Unicode) {
        Ok(text) if text == sentinel => "our Sentinel, untouched — the game never copied".into(),
        Ok(text) if text.is_empty() => "no text at all".into(),
        Ok(text) => format!(
            "{} chars of something else — {:?}…",
            text.chars().count(),
            text.chars().take(40).collect::<String>()
        ),
        Err(err) => format!("nothing readable ({err})"),
    }
}

// ---------------------------------------------------------------------------------------------
// Injection
// ---------------------------------------------------------------------------------------------

fn click_left() -> Result<(), PlatformError> {
    let down = mouse_input(MOUSEEVENTF_LEFTDOWN);
    let up = mouse_input(MOUSEEVENTF_LEFTUP);
    send(&[down, up], "SendInput(left click)")
}

/// One `Ctrl+C`, with Shift left exactly as the human is holding it.
///
/// The clipboard research recommended releasing held modifiers before copying, and the spike proved
/// that unnecessary: `Ctrl+C` works with Shift held from our own `SendInput`. Releasing it would end
/// Apply Mode every single Roll, so copy mode B is dead.
fn send_copy() -> Result<(), PlatformError> {
    let events = [
        key_input(VK_CONTROL, false),
        key_input(VK_C, false),
        key_input(VK_C, true),
        key_input(VK_CONTROL, true),
    ];
    send(&events, "SendInput(Ctrl+C)")
}

fn mouse_input(flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key_input(key: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(events: &[INPUT], capability: &'static str) -> Result<(), PlatformError> {
    // SAFETY: `events` is a live slice of correctly initialised `INPUT` values, and `cbSize` is the
    // size Windows expects. `SendInput` reads it and returns.
    let sent = unsafe { SendInput(events, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == events.len() {
        Ok(())
    } else {
        Err(PlatformError::Os {
            capability,
            detail: format!(
                "injected {sent} of {} events — UIPI blocks injection into a window running at a \
                 higher integrity level, which is what this looks like when the game is elevated and \
                 we are not",
                events.len()
            ),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------------------------

fn cursor_pos() -> Result<(i32, i32), PlatformError> {
    let mut point = POINT::default();
    // SAFETY: `GetCursorPos` writes two `i32`s into a `POINT` we own and keep alive.
    unsafe { GetCursorPos(&mut point) }.map_err(|err| PlatformError::Os {
        capability: "GetCursorPos",
        detail: err.message(),
    })?;
    Ok((point.x, point.y))
}

fn key_down(vk: i32) -> bool {
    // SAFETY: a pure read of global key state for a valid virtual-key code.
    (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0
}

/// The foreground window's class name, and a description for the log.
///
/// The class is what core judges, because [#17](https://github.com/Furizaa/poe-graft/issues/17)
/// measured it as `POEWindowClass`. The title comes along for the log only — a Refusal that says
/// "not Path of Exile" is far more useful next to what *was* in front.
fn foreground() -> (String, String) {
    // SAFETY: `GetForegroundWindow` is a pure read and may return a null handle, which the two text
    // calls then simply fail on, returning 0 length.
    let hwnd = unsafe { GetForegroundWindow() };
    let mut title = [0u16; 256];
    let mut class = [0u16; 256];
    // SAFETY: both take a slice they will not write past; the bindings pass the length.
    let title_len = unsafe { GetWindowTextW(hwnd, &mut title) }.max(0) as usize;
    let class_len = unsafe { GetClassNameW(hwnd, &mut class) }.max(0) as usize;

    let class = String::from_utf16_lossy(&class[..class_len]);
    let described = format!(
        "{:?} [{class}]",
        String::from_utf16_lossy(&title[..title_len])
    );
    (class, described)
}

/// Read the three accessibility settings that change how a held Shift behaves.
pub fn accessibility() -> Result<Accessibility, PlatformError> {
    let mut sticky = STICKYKEYS {
        cbSize: std::mem::size_of::<STICKYKEYS>() as u32,
        ..Default::default()
    };
    let mut filter = FILTERKEYS {
        cbSize: std::mem::size_of::<FILTERKEYS>() as u32,
        ..Default::default()
    };
    let mut toggle = TOGGLEKEYS {
        cbSize: std::mem::size_of::<TOGGLEKEYS>() as u32,
        ..Default::default()
    };

    // SAFETY: each call is a documented read into a correctly sized struct we own, with `cbSize` set
    // as Windows requires and the matching SPI_GET* action.
    unsafe {
        read_spi(
            SPI_GETSTICKYKEYS,
            sticky.cbSize,
            &mut sticky as *mut _ as *mut _,
        )?;
        read_spi(
            SPI_GETFILTERKEYS,
            filter.cbSize,
            &mut filter as *mut _ as *mut _,
        )?;
        read_spi(
            SPI_GETTOGGLEKEYS,
            toggle.cbSize,
            &mut toggle as *mut _ as *mut _,
        )?;
    }

    Ok(Accessibility {
        sticky_keys_on: sticky.dwFlags.contains(SKF_STICKYKEYSON),
        sticky_keys_available: sticky.dwFlags.contains(SKF_AVAILABLE),
        filter_keys_on: filter.dwFlags & FKF_FILTERKEYSON != 0,
        toggle_keys_on: toggle.dwFlags & TKF_TOGGLEKEYSON != 0,
    })
}

/// # Safety
///
/// `buffer` must point at a writable struct of at least `size` bytes matching `action`.
unsafe fn read_spi(
    action: windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_ACTION,
    size: u32,
    buffer: *mut core::ffi::c_void,
) -> Result<(), PlatformError> {
    unsafe {
        SystemParametersInfoW(
            action,
            size,
            Some(buffer),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .map_err(|err| PlatformError::Os {
        capability: "SystemParametersInfoW",
        detail: err.message(),
    })
}

fn log_accessibility() {
    match accessibility() {
        Ok(state) => log(&format!(
            "cycle: sticky keys {} (available {}) · filter keys {} · toggle keys {}",
            onoff(state.sticky_keys_on),
            onoff(state.sticky_keys_available),
            onoff(state.filter_keys_on),
            onoff(state.toggle_keys_on),
        )),
        Err(err) => log(&format!(
            "cycle: could not read the accessibility settings: {err}"
        )),
    }
}

fn onoff(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "off"
    }
}

// ---------------------------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------------------------

/// Take a lock, ignoring poisoning. A panic elsewhere must not make the app unusable on the one
/// machine that can run it.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// `"[ { (0xDB)"`. Covers the keys plausible as a Trigger Key on a keyboard with no F-row; anything
/// else still reads back as a code the human can act on.
pub fn describe_vk(vk: u32) -> String {
    if vk == 0 {
        return "none".to_string();
    }
    let name = match vk {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x13 => "Pause",
        0x14 => "Caps Lock",
        0x1B => "Esc",
        0x20 => "Space",
        0x21 => "Page Up",
        0x22 => "Page Down",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2D => "Insert",
        0x2E => "Delete",
        0x30..=0x39 => return format!("{} (0x{vk:02X})", (b'0' + (vk - 0x30) as u8) as char),
        0x41..=0x5A => return format!("{} (0x{vk:02X})", (b'A' + (vk - 0x41) as u8) as char),
        0x5B => "Left Windows",
        0x5D => "Menu",
        0x60..=0x69 => return format!("Numpad {} (0x{vk:02X})", vk - 0x60),
        0x6A => "Numpad *",
        0x6B => "Numpad +",
        0x6D => "Numpad -",
        0x6E => "Numpad .",
        0x6F => "Numpad /",
        0x70..=0x87 => return format!("F{} (0x{vk:02X})", vk - 0x6F),
        0x90 => "Num Lock",
        0x91 => "Scroll Lock",
        0xA0 => "Left Shift",
        0xA1 => "Right Shift",
        0xA2 => "Left Ctrl",
        0xA3 => "Right Ctrl",
        0xA4 => "Left Alt",
        0xA5 => "Right Alt",
        0xBA => "; :",
        0xBB => "= +",
        0xBC => ", <",
        0xBD => "- _",
        0xBE => ". >",
        0xBF => "/ ?",
        0xC0 => "` ~",
        0xDB => "[ {",
        0xDC => "\\ |",
        0xDD => "] }",
        0xDE => "' \"",
        _ => "unknown",
    };
    format!("{name} (0x{vk:02X})")
}
