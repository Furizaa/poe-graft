//! Throwaway on-device spike for
//! [#17](https://github.com/Furizaa/poe-graft/issues/17).
//!
//! **This code is not the app.** It exists to settle five questions on the gaming PC that the
//! roll cycle would otherwise have to assume, and
//! [Design the roll cycle and the hit latch](https://github.com/Furizaa/poe-graft/issues/7)
//! designs the real thing afterwards. Nothing here should be read as pre-empting the cycle's
//! state names, and none of it has to survive.
//!
//! # The TOS rule this obeys
//!
//! Exactly **one** injected left-click and **one** `Ctrl+C` per trigger press the human
//! physically makes, and nothing otherwise. No timer, no repeat without a fresh press, nothing
//! done in reaction to what was read. Three mechanisms enforce that rather than merely
//! intending it:
//!
//! * `LLKHF_INJECTED` presses are ignored, so the spike can never react to its own input.
//! * Auto-repeat is filtered by tracking the key's own up/down state, so holding the trigger
//!   down is one press, not a stream of them.
//! * A cycle already in flight makes the next press a no-op rather than a queued action, and
//!   the count of presses dropped that way is recorded — this is ADR 0001's fail-closed
//!   sequencing rule, observable on device.
//!
//! # The hook callback's budget
//!
//! Every keyboard event on the desktop waits inside [`keyboard_hook`]. The budget is 300 ms
//! (`LowLevelHooksTimeout`), and on the 11th overrun Windows **silently uninstalls the hook**
//! with no way for the app to notice — it fails *open*. So the callback does nothing but relaxed
//! atomic loads, integer comparisons and one `fetch_add`. No allocation, no locks, no I/O, no
//! logging, and above all no injection: that happens on a worker thread.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use poe_graft_core::PlatformError;
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
    CallNextHookEx, GetClassNameW, GetCursorPos, GetForegroundWindow, GetMessageW,
    GetWindowTextW, PostThreadMessageW, SetWindowsHookExW, SystemParametersInfoW,
    UnhookWindowsHookEx, FKF_FILTERKEYSON, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
    SPI_GETFILTERKEYS, SPI_GETSTICKYKEYS, SPI_GETTOGGLEKEYS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    TKF_TOGGLEKEYSON, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

// ---------------------------------------------------------------------------------------------
// Where the spike's findings go
// ---------------------------------------------------------------------------------------------

/// Somewhere to put a line of text. `src-tauri` supplies one backed by the app's journal.
pub type LogSink = Box<dyn Fn(&str) + Send + Sync>;

/// The log sink, installed once by `src-tauri`.
///
/// Everything the spike learns has to reach the **file**, not just the window: the updater
/// force-exits the app on Windows, and a crashing hook takes the window with it. A finding that
/// only ever rendered is a finding lost.
static SINK: OnceLock<LogSink> = OnceLock::new();

/// Point the spike's logging at the app's journal. Later calls are ignored.
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

/// Virtual-key code of the trigger. `0` means no trigger chosen, and the hook does nothing.
static TRIGGER_VK: AtomicU32 = AtomicU32::new(0);
/// Whether to swallow the trigger key so it never reaches the game.
static SUPPRESS: AtomicBool = AtomicBool::new(false);
/// Whether a trigger press should start a cycle.
static ARMED: AtomicBool = AtomicBool::new(false);
/// Whether to record key codes for the "which key is this?" readout.
static LEARNING: AtomicBool = AtomicBool::new(false);
/// Last physical key code seen while [`LEARNING`].
static LAST_KEY_VK: AtomicU32 = AtomicU32::new(0);
/// How many physical key-down events the callback has observed since the hook went in.
///
/// A **count only** — never which keys, unless [`LEARNING`] is on. It exists because
/// `SetWindowsHookExW` returning a valid handle proves only that Windows accepted the hook, not
/// that it is delivering anything: a hook that installs cleanly and then hears nothing looks
/// identical, from the app, to a panel that is failing to render what it heard. This number tells
/// those two apart in one glance, which is the difference between a diagnosis and a guess.
static KEYS_SEEN: AtomicU32 = AtomicU32::new(0);
/// Tracks the trigger key's own up/down state so auto-repeat is not mistaken for a new press.
static TRIGGER_DOWN: AtomicBool = AtomicBool::new(false);
/// Monotonic count of fresh physical trigger presses seen while armed. The worker compares this
/// against the number it has handled, which is how dropped presses become a recorded number
/// rather than a silent gap.
static PRESS_SEQ: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------------------------
// State only the worker and the commands touch
// ---------------------------------------------------------------------------------------------

/// Settle time between the injected click and `Ctrl+C`, so the copy reads the *new* roll.
static COPY_DELAY_MS: AtomicU32 = AtomicU32::new(40);
/// How long to wait for the clipboard to change before calling the read a timeout.
static READ_TIMEOUT_MS: AtomicU32 = AtomicU32::new(500);
/// How far the cursor may drift from the captured item position before the spike refuses.
static TOLERANCE_PX: AtomicI32 = AtomicI32::new(24);
/// Hard ceiling on rolls per arming. Currency is real; a runaway loop is the risk this bounds.
static MAX_ROLLS: AtomicU32 = AtomicU32::new(60);
/// Copy mode B: drop Shift for the duration of `Ctrl+C`, then put it back.
static RELEASE_SHIFT: AtomicBool = AtomicBool::new(false);
/// Refuse to inject unless Path of Exile is the foreground window.
static GUARD_FOREGROUND: AtomicBool = AtomicBool::new(true);

/// The captured item position, and whether it has been captured at all.
static POS_X: AtomicI32 = AtomicI32::new(0);
static POS_Y: AtomicI32 = AtomicI32::new(0);
static POS_SET: AtomicBool = AtomicBool::new(false);

/// Rolls injected since the last arming.
static ROLLS: AtomicU32 = AtomicU32::new(0);
/// Consecutive reads that told us nothing. See [`BAD_LIMIT`].
static CONSECUTIVE_BAD: AtomicU32 = AtomicU32::new(0);

/// How many unreadable rolls in a row before the spike disarms itself.
///
/// A read that times out or comes back stale means the app has **no idea** what state the game is
/// in, and injecting more clicks blind is the runaway case
/// [The safety contract](https://github.com/Furizaa/poe-graft/issues/8) is about. It is also the
/// signature of the specific accident that matters here: if Shift-persist fails, the orb leaves
/// the cursor and the next click picks the jewel *up* — after which nothing is hovered, `Ctrl+C`
/// copies nothing, and every read times out. Stopping after three bounds that to three clicks.
const BAD_LIMIT: u32 = 3;
/// Makes each poison string unique, so a stale read can never look like a fresh one.
static SENTINEL_SEQ: AtomicU32 = AtomicU32::new(0);

/// Set while the worker thread should keep running.
static WORKER_RUN: AtomicBool = AtomicBool::new(false);

/// What the last completed roll looked like, for the UI.
static LAST_ROLL: Mutex<Option<RollRecord>> = Mutex::new(None);
/// The previous roll's raw text, to notice two identical reads in a row.
static PREV_TEXT: Mutex<Option<String>> = Mutex::new(None);
/// The installed hook's threads, so it can be taken down again.
static HOOK: Mutex<Option<Installed>> = Mutex::new(None);

struct Installed {
    hook_thread_id: u32,
    hook_thread: JoinHandle<()>,
    worker: JoinHandle<()>,
}

// ---------------------------------------------------------------------------------------------
// Readouts handed up to `src-tauri`
// ---------------------------------------------------------------------------------------------

/// What one completed roll produced. No `serde` derive: this crate stays free of it, and the DTO
/// lives in `src-tauri` — the same seam discipline as `PlatformInfo`.
#[derive(Debug, Clone)]
pub struct RollRecord {
    /// Which roll of this arming, 1-based.
    pub roll: u32,
    /// `Ctrl+C` → clipboard changed. The number to compare against the 15–32 ms AutoHotkey saw.
    pub copy_ms: u32,
    /// Click → text in hand, i.e. `copy_ms` plus the configured settle delay.
    pub cycle_ms: u32,
    /// The clipboard never changed inside the timeout.
    pub timed_out: bool,
    /// The clipboard changed but still held the sentinel — something else wrote to it.
    pub stale: bool,
    /// Byte-identical to the previous roll's text, which usually means the settle delay is too
    /// short and the copy is reading the item as it was *before* the click.
    pub identical_to_previous: bool,
    /// Whether Shift was physically down when the cycle started.
    pub shift_down: bool,
    /// Length of the captured text, as a cheap "did we get a whole item" signal.
    pub chars: usize,
    /// The first modifier line of the captured item, for the UI. The log holds the full text.
    pub summary: String,
}

/// Accessibility settings that silently change how held modifiers behave.
#[derive(Debug, Clone, Copy)]
pub struct Accessibility {
    /// Sticky Keys is **on**. This is the one that silently breaks Shift-persist.
    pub sticky_keys_on: bool,
    /// Sticky Keys is available to be switched on — e.g. by the five-taps-on-Shift shortcut,
    /// which is exactly the gesture a Shift-heavy crafting session might trip by accident.
    pub sticky_keys_available: bool,
    /// Filter Keys is on: it drops or delays repeated keystrokes.
    pub filter_keys_on: bool,
    /// Toggle Keys is on: harmless here, but it confirms the read is working.
    pub toggle_keys_on: bool,
}

/// Everything the spike panel shows.
#[derive(Debug, Clone)]
pub struct SpikeStatus {
    pub hook_installed: bool,
    pub armed: bool,
    pub learning: bool,
    pub suppress: bool,
    pub release_shift: bool,
    pub guard_foreground: bool,
    pub trigger_vk: u32,
    pub trigger_name: String,
    pub last_key_vk: u32,
    pub last_key_name: String,
    /// Physical key-downs the callback has observed. Zero while the hook is installed means the
    /// hook is deaf, which is a different fault from anything the panel could be getting wrong.
    pub keys_seen: u32,
    pub position: Option<(i32, i32)>,
    pub rolls: u32,
    pub max_rolls: u32,
    pub copy_delay_ms: u32,
    pub read_timeout_ms: u32,
    pub tolerance_px: i32,
    pub presses: u32,
    pub shift_down: bool,
    pub foreground: String,
    pub last_roll: Option<RollRecord>,
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
    // Our own `SendInput` traffic comes back through this hook. Ignoring it is what makes "one
    // action per *physical* press" a property of the code rather than a hope.
    let injected = event.flags.contains(LLKHF_INJECTED);

    if is_down && !injected {
        KEYS_SEEN.fetch_add(1, Ordering::Relaxed);
        if LEARNING.load(Ordering::Relaxed) {
            LAST_KEY_VK.store(event.vkCode, Ordering::Relaxed);
        }
    }

    let trigger = TRIGGER_VK.load(Ordering::Relaxed);
    if trigger == 0 || event.vkCode != trigger || injected {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    if is_down {
        // `swap` returning false means the key was up, so this is a fresh press rather than the
        // auto-repeat stream Windows sends while a key is held.
        if !TRIGGER_DOWN.swap(true, Ordering::Relaxed) && ARMED.load(Ordering::Relaxed) {
            PRESS_SEQ.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        TRIGGER_DOWN.store(false, Ordering::Relaxed);
    }

    if SUPPRESS.load(Ordering::Relaxed) {
        // Non-zero: the event dies here and no other hook or application sees it. Whether that
        // reaches Path of Exile, which reads the mouse via Raw Input, is one of the questions.
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

// ---------------------------------------------------------------------------------------------
// Install / uninstall
// ---------------------------------------------------------------------------------------------

/// Install the keyboard hook and start the worker.
///
/// The hook is installed *on the thread that pumps messages for it*, which is a Win32
/// requirement, not a style choice: a low-level hook is serviced by its owning thread's message
/// loop, so a hook installed on a thread that never calls `GetMessageW` silently never fires.
pub fn install() -> Result<(), PlatformError> {
    let mut slot = lock(&HOOK);
    if slot.is_some() {
        return Ok(());
    }

    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();

    let hook_thread = std::thread::Builder::new()
        .name("poe-graft-hook".into())
        .spawn(move || {
            // SAFETY: `keyboard_hook` is a `'static` function in this process. `None` for the
            // module handle is correct for a low-level hook whose procedure lives in the calling
            // process; thread id 0 makes it global.
            let hook = match unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0)
            } {
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

            // Servicing loop. A low-level hook is dispatched by its owning thread's message
            // pump, so this loop existing is what makes the hook fire at all. There is no
            // window on this thread, so there is nothing to translate or dispatch — retrieving
            // the message is the whole job.
            //
            // `uninstall` ends it with a posted WM_QUIT, for which `GetMessageW` returns 0. It
            // returns -1 on error, and treating that as "keep going" would spin a core forever,
            // so anything not strictly positive stops the loop.
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
                log(&format!("spike: unhook failed: {}", err.message()));
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
                detail: "hook thread died before reporting".into(),
            });
        }
    };

    // Reset both so "installed, and it has seen 0 keys" is unambiguous rather than a leftover
    // from a previous install in the same session.
    KEYS_SEEN.store(0, Ordering::Relaxed);
    LAST_KEY_VK.store(0, Ordering::Relaxed);

    WORKER_RUN.store(true, Ordering::Release);
    let worker = std::thread::Builder::new()
        .name("poe-graft-spike".into())
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

    log("spike: WH_KEYBOARD_LL installed");
    log_accessibility();
    Ok(())
}

/// Take the hook down and stop the worker. Also disarms — an uninstalled hook that still
/// believed itself armed would be the worst kind of confusing.
pub fn uninstall() -> Result<(), PlatformError> {
    let installed = lock(&HOOK).take();
    let Some(installed) = installed else {
        return Ok(());
    };

    ARMED.store(false, Ordering::Relaxed);
    WORKER_RUN.store(false, Ordering::Release);

    // SAFETY: posting a message to a thread id is safe; a dead thread just makes it fail.
    if let Err(err) = unsafe {
        PostThreadMessageW(installed.hook_thread_id, WM_QUIT, WPARAM(0), LPARAM(0))
    } {
        log(&format!("spike: could not post WM_QUIT: {}", err.message()));
    }

    let _ = installed.hook_thread.join();
    let _ = installed.worker.join();
    log("spike: WH_KEYBOARD_LL removed");
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The worker: one cycle per physical press
// ---------------------------------------------------------------------------------------------

fn worker_loop() {
    let mut handled = PRESS_SEQ.load(Ordering::Relaxed);
    let mut reported_key = LAST_KEY_VK.load(Ordering::Relaxed);

    while WORKER_RUN.load(Ordering::Acquire) {
        // The callback cannot log — it has a 300 ms budget and must not allocate — so the durable
        // record of a learned key is written from here. It doubles as proof this worker is alive,
        // which every roll also depends on: if keys are being seen but nothing appears here, the
        // fault is the worker, not the hook.
        let key = LAST_KEY_VK.load(Ordering::Relaxed);
        if key != reported_key {
            reported_key = key;
            if key != 0 {
                log(&format!("spike: saw key {}", describe_vk(key)));
            }
        }

        let seq = PRESS_SEQ.load(Ordering::Relaxed);
        if seq == handled {
            // Idle. Sleep granularity is coarse on Windows and it does not matter here — this
            // only adds a millisecond or two to a human-paced press, and every measurement that
            // matters is taken with `Instant` around a busy wait instead.
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        // Everything between `handled` and `seq` beyond the one we are about to run arrived
        // while the previous cycle was still in flight, and was deliberately dropped.
        let dropped = seq - handled - 1;
        handled = seq;
        on_trigger(dropped);
    }
}

fn on_trigger(dropped: u32) {
    if dropped > 0 {
        log(&format!(
            "spike: {dropped} trigger press(es) ignored — a cycle was still in flight (fail-closed)"
        ));
    }

    let foreground = foreground_window();

    // The first press after arming captures where the item is and injects nothing. This is the
    // only way to capture it: the cursor has to be over the item in the game, so it cannot also
    // be over a button in our window.
    if !POS_SET.load(Ordering::Relaxed) {
        match cursor_pos() {
            Ok((x, y)) => {
                POS_X.store(x, Ordering::Relaxed);
                POS_Y.store(y, Ordering::Relaxed);
                POS_SET.store(true, Ordering::Release);
                log(&format!(
                    "spike: captured item position {x},{y} · foreground {foreground} — press again to roll"
                ));
            }
            Err(err) => log(&format!("spike: could not capture position: {err}")),
        }
        return;
    }

    let rolls = ROLLS.load(Ordering::Relaxed);
    let cap = MAX_ROLLS.load(Ordering::Relaxed);
    if rolls >= cap {
        ARMED.store(false, Ordering::Relaxed);
        log(&format!(
            "spike: refused and disarmed — roll cap {cap} reached. Re-arm to continue."
        ));
        return;
    }

    if GUARD_FOREGROUND.load(Ordering::Relaxed) && !looks_like_poe(&foreground) {
        log(&format!(
            "spike: refused — foreground window is {foreground}, not Path of Exile"
        ));
        return;
    }

    let (cx, cy) = match cursor_pos() {
        Ok(pos) => pos,
        Err(err) => {
            log(&format!("spike: refused — could not read cursor: {err}"));
            return;
        }
    };
    let (px, py) = (POS_X.load(Ordering::Relaxed), POS_Y.load(Ordering::Relaxed));
    let (dx, dy) = ((cx - px).abs(), (cy - py).abs());
    let tolerance = TOLERANCE_PX.load(Ordering::Relaxed);
    if dx > tolerance || dy > tolerance {
        log(&format!(
            "spike: refused — cursor {cx},{cy} is {dx},{dy} px off the captured {px},{py} \
             (tolerance {tolerance}). Hover the item again, or forget the position to recapture."
        ));
        return;
    }

    // Shift up means apply mode is not active, and a plain left-click on an inventory item
    // *picks it up*. Refusing is both the safe answer and the honest one: if Shift-persist
    // drops out mid-session, this is the line in the log that says so.
    let shift_down = key_down(VK_SHIFT.0 as i32);
    if !shift_down {
        log("spike: refused — Shift is not held, so a click would pick the item up rather than \
             apply the orb. Hold Shift with the orbs in your inventory.");
        return;
    }

    run_cycle(rolls + 1, shift_down);
}

/// One injected click, one `Ctrl+C`, one read. Nothing conditional on what comes back.
fn run_cycle(roll: u32, shift_down: bool) {
    let sentinel = format!(
        "poe-graft-sentinel-{}",
        SENTINEL_SEQ.fetch_add(1, Ordering::Relaxed)
    );

    // Poison first. #3 settled that content comparison cannot prove freshness — two identical
    // rolls produce identical bytes — so the clipboard has to hold something that could only
    // have come from us before the copy is asked for.
    if let Err(err) = clipboard_win::set_clipboard(clipboard_win::formats::Unicode, &sentinel) {
        log(&format!("spike: refused — could not poison the clipboard: {err}"));
        return;
    }
    let poisoned_seq = clipboard_win::seq_num().map(|n| n.get()).unwrap_or(0);

    let delay = COPY_DELAY_MS.load(Ordering::Relaxed);
    let timeout = Duration::from_millis(READ_TIMEOUT_MS.load(Ordering::Relaxed) as u64);
    let release_shift = RELEASE_SHIFT.load(Ordering::Relaxed);

    let cycle_started = Instant::now();

    // The one game action. No move flag and no coordinates, so this lands wherever the cursor
    // already is and the cursor never moves — which is what keeps the app out of the
    // "acts in reaction to a read" category the policy prohibits.
    if let Err(err) = click_left() {
        log(&format!("spike: click failed: {err}"));
        return;
    }

    // The roll is counted here, the instant the click lands — not once a result comes back. An
    // orb is spent whether or not the read afterwards succeeds, times out or fails outright, and
    // a counter that only advanced on success would let the roll cap be walked straight past by
    // a run of failed reads.
    ROLLS.store(roll, Ordering::Relaxed);

    // Let the client process the click and rebuild the item's tooltip before copying it.
    spin_for(Duration::from_millis(delay as u64));

    let copy_started = Instant::now();
    if let Err(err) = send_copy(release_shift) {
        log(&format!("spike: Ctrl+C failed: {err}"));
        return;
    }

    // Poll the sequence number rather than the contents: `GetClipboardSequenceNumber` needs no
    // open clipboard handle, so this never contends with the game for the clipboard lock. Busy
    // waiting, because the answer is expected in tens of milliseconds and `sleep` on Windows
    // rounds to the scheduler tick, which would swamp the measurement.
    let mut timed_out = true;
    while copy_started.elapsed() < timeout {
        if clipboard_win::seq_num().map(|n| n.get()).unwrap_or(0) != poisoned_seq {
            timed_out = false;
            break;
        }
        std::hint::spin_loop();
    }
    let copy_ms = copy_started.elapsed().as_millis() as u32;

    if timed_out {
        log(&format!(
            "=== roll {roll}: TIMEOUT after {copy_ms}ms (delay {delay}ms) ==="
        ));
        note_bad_read();
        store_roll(RollRecord {
            roll,
            copy_ms,
            cycle_ms: cycle_started.elapsed().as_millis() as u32,
            timed_out: true,
            stale: false,
            identical_to_previous: false,
            shift_down,
            chars: 0,
            summary: String::new(),
        });
        return;
    }

    let text = match clipboard_win::get_clipboard::<String, _>(clipboard_win::formats::Unicode) {
        Ok(text) => text,
        Err(err) => {
            log(&format!("=== roll {roll}: READ FAILED after {copy_ms}ms: {err} ==="));
            note_bad_read();
            return;
        }
    };
    let cycle_ms = cycle_started.elapsed().as_millis() as u32;

    let stale = text == sentinel;
    let mut previous = lock(&PREV_TEXT);
    let identical_to_previous = previous.as_deref() == Some(text.as_str());
    *previous = Some(text.clone());
    drop(previous);

    if stale {
        note_bad_read();
    } else {
        CONSECUTIVE_BAD.store(0, Ordering::Relaxed);
    }

    let outcome = if stale {
        "STALE (clipboard changed but still held the sentinel)"
    } else if identical_to_previous {
        "OK but IDENTICAL to the previous roll"
    } else {
        "OK"
    };
    // Full raw text, deliberately. These are the fixtures the parser will be tested against,
    // and this is the only machine that can produce them.
    log(&format!(
        "=== roll {roll}: {outcome} in {copy_ms}ms (cycle {cycle_ms}ms, delay {delay}ms, \
         {} chars, shift {}) ===\n{text}\n=== end roll {roll} ===",
        text.chars().count(),
        if shift_down { "down" } else { "UP" }
    ));

    store_roll(RollRecord {
        roll,
        copy_ms,
        cycle_ms,
        timed_out: false,
        stale,
        identical_to_previous,
        shift_down,
        chars: text.chars().count(),
        summary: summarise(&text),
    });
}

/// Count an unreadable roll, and disarm if they are stacking up. See [`BAD_LIMIT`].
fn note_bad_read() {
    let bad = CONSECUTIVE_BAD.fetch_add(1, Ordering::Relaxed) + 1;
    if bad >= BAD_LIMIT {
        ARMED.store(false, Ordering::Relaxed);
        log(&format!(
            "──── spike DISARMED ITSELF: {bad} reads in a row told us nothing. The app cannot see \
             what state the game is in, so it has stopped rather than keep clicking. Check that \
             the jewel is still in the inventory and still hovered, that an Orb of Alteration is \
             still on the cursor, and that Shift is still held. ────"
        ));
    }
}

/// The first modifier-looking line, for the panel. Best effort — the log holds the whole thing.
fn summarise(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| line.contains("(Tier:"))
        .or_else(|| text.lines().map(str::trim).find(|line| !line.is_empty()))
        .unwrap_or("")
        .chars()
        .take(120)
        .collect()
}

fn store_roll(record: RollRecord) {
    *lock(&LAST_ROLL) = Some(record);
}

// ---------------------------------------------------------------------------------------------
// Injection
// ---------------------------------------------------------------------------------------------

fn click_left() -> Result<(), PlatformError> {
    let down = mouse_input(MOUSEEVENTF_LEFTDOWN);
    let up = mouse_input(MOUSEEVENTF_LEFTUP);
    send(&[down, up], "SendInput(left click)")
}

fn send_copy(release_shift: bool) -> Result<(), PlatformError> {
    let mut events = Vec::with_capacity(6);
    // Copy mode B. The clipboard research recommends releasing held modifiers before copying,
    // but doing so ends apply mode every roll — so mode A is the default and this is the
    // fallback the spike exists to compare it against.
    if release_shift {
        events.push(key_input(VK_SHIFT, true));
    }
    events.push(key_input(VK_CONTROL, false));
    events.push(key_input(VK_C, false));
    events.push(key_input(VK_C, true));
    events.push(key_input(VK_CONTROL, true));
    if release_shift {
        events.push(key_input(VK_SHIFT, false));
    }
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

fn key_input(
    key: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
    up: bool,
) -> INPUT {
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
    // SAFETY: `events` is a live slice of correctly initialised `INPUT` values, and `cbSize` is
    // the size Windows expects. `SendInput` reads it and returns.
    let sent = unsafe { SendInput(events, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == events.len() {
        Ok(())
    } else {
        Err(PlatformError::Os {
            capability,
            detail: format!(
                "injected {sent} of {} events — UIPI blocks injection into a window running at a \
                 higher integrity level, which is what this looks like when the game is elevated \
                 and we are not",
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

/// `"title" [ClassName]` for the foreground window, for the log and the drift guard.
fn foreground_window() -> String {
    // SAFETY: `GetForegroundWindow` is a pure read and may return a null handle, which the two
    // text calls then simply fail on, returning 0 length.
    let hwnd = unsafe { GetForegroundWindow() };
    let mut title = [0u16; 256];
    let mut class = [0u16; 256];
    // SAFETY: both take a slice they will not write past; the bindings pass the length.
    let title_len = unsafe { GetWindowTextW(hwnd, &mut title) }.max(0) as usize;
    let class_len = unsafe { GetClassNameW(hwnd, &mut class) }.max(0) as usize;
    format!(
        "{:?} [{}]",
        String::from_utf16_lossy(&title[..title_len]),
        String::from_utf16_lossy(&class[..class_len])
    )
}

/// Deliberately a title match rather than a hardcoded class name: the class differs between the
/// standalone and Steam clients, and being wrong here would refuse every roll. The log records
/// the real title and class so the next session can tighten this with evidence.
fn looks_like_poe(foreground: &str) -> bool {
    foreground.to_lowercase().contains("path of exile")
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

    // SAFETY: each call is a documented read into a correctly sized struct we own, with `cbSize`
    // set as Windows requires and the matching SPI_GET* action.
    unsafe {
        read_spi(SPI_GETSTICKYKEYS, sticky.cbSize, &mut sticky as *mut _ as *mut _)?;
        read_spi(SPI_GETFILTERKEYS, filter.cbSize, &mut filter as *mut _ as *mut _)?;
        read_spi(SPI_GETTOGGLEKEYS, toggle.cbSize, &mut toggle as *mut _ as *mut _)?;
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
        SystemParametersInfoW(action, size, Some(buffer), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
    }
    .map_err(|err| PlatformError::Os {
        capability: "SystemParametersInfoW",
        detail: err.message(),
    })
}

fn log_accessibility() {
    match accessibility() {
        Ok(state) => log(&format!(
            "spike: sticky keys {} (available {}) · filter keys {} · toggle keys {}",
            onoff(state.sticky_keys_on),
            onoff(state.sticky_keys_available),
            onoff(state.filter_keys_on),
            onoff(state.toggle_keys_on),
        )),
        Err(err) => log(&format!("spike: could not read accessibility settings: {err}")),
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
// Controls
// ---------------------------------------------------------------------------------------------

/// Choose the trigger key. `0` disables the trigger entirely.
pub fn set_trigger(vk: u32) {
    TRIGGER_VK.store(vk, Ordering::Relaxed);
    TRIGGER_DOWN.store(false, Ordering::Relaxed);
    log(&format!("spike: trigger key set to {}", describe_vk(vk)));
}

/// Turn key-code recording on or off.
pub fn set_learning(on: bool) {
    LEARNING.store(on, Ordering::Relaxed);
    log(if on {
        "spike: learning keys — the next key you press will be reported"
    } else {
        "spike: stopped learning keys"
    });
}

/// Swallow the trigger key so it never reaches the game.
pub fn set_suppress(on: bool) {
    SUPPRESS.store(on, Ordering::Relaxed);
    log(&format!("spike: trigger-key suppression {}", onoff(on)));
}

/// Copy mode: release Shift around `Ctrl+C` (mode B) or leave it held (mode A, the default).
pub fn set_release_shift(on: bool) {
    RELEASE_SHIFT.store(on, Ordering::Relaxed);
    log(&format!(
        "spike: copy mode {}",
        if on {
            "B — Shift released around Ctrl+C"
        } else {
            "A — Shift stays held through Ctrl+C"
        }
    ));
}

/// Whether to refuse unless Path of Exile is in the foreground.
pub fn set_guard_foreground(on: bool) {
    GUARD_FOREGROUND.store(on, Ordering::Relaxed);
    log(&format!("spike: foreground guard {}", onoff(on)));
}

/// Timings and bounds. Each is clamped to something that cannot brick the session.
pub fn set_timing(copy_delay_ms: u32, read_timeout_ms: u32, tolerance_px: i32, max_rolls: u32) {
    let copy_delay_ms = copy_delay_ms.min(2_000);
    let read_timeout_ms = read_timeout_ms.clamp(50, 5_000);
    let tolerance_px = tolerance_px.clamp(0, 400);
    let max_rolls = max_rolls.clamp(1, 1_000);

    COPY_DELAY_MS.store(copy_delay_ms, Ordering::Relaxed);
    READ_TIMEOUT_MS.store(read_timeout_ms, Ordering::Relaxed);
    TOLERANCE_PX.store(tolerance_px, Ordering::Relaxed);
    MAX_ROLLS.store(max_rolls, Ordering::Relaxed);
    log(&format!(
        "spike: delay {copy_delay_ms}ms · read timeout {read_timeout_ms}ms · \
         tolerance {tolerance_px}px · cap {max_rolls} rolls"
    ));
}

/// Arm or disarm. Arming resets the roll count and forgets the captured position, so the first
/// press of every session recaptures it — a stale coordinate from a previous window layout is
/// exactly the mistake worth making impossible.
pub fn set_armed(on: bool) -> Result<(), PlatformError> {
    if on && lock(&HOOK).is_none() {
        return Err(PlatformError::Os {
            capability: "arm",
            detail: "the keyboard hook is not installed".into(),
        });
    }

    if on {
        ROLLS.store(0, Ordering::Relaxed);
        CONSECUTIVE_BAD.store(0, Ordering::Relaxed);
        POS_SET.store(false, Ordering::Relaxed);
        *lock(&PREV_TEXT) = None;
        *lock(&LAST_ROLL) = None;
        // PRESS_SEQ is deliberately *not* reset. It is a monotonic counter the worker compares
        // against its own tally, so rewinding it would make the worker see a backwards jump and
        // either fire a phantom cycle or go deaf. It only counts up while armed, so after a
        // disarm the two are already level.
        ARMED.store(true, Ordering::Release);
        log(&format!(
            "──── spike armed · trigger {} · delay {}ms · timeout {}ms · tolerance {}px · \
             cap {} rolls · copy mode {} · suppression {} · foreground guard {} ────",
            describe_vk(TRIGGER_VK.load(Ordering::Relaxed)),
            COPY_DELAY_MS.load(Ordering::Relaxed),
            READ_TIMEOUT_MS.load(Ordering::Relaxed),
            TOLERANCE_PX.load(Ordering::Relaxed),
            MAX_ROLLS.load(Ordering::Relaxed),
            if RELEASE_SHIFT.load(Ordering::Relaxed) { "B" } else { "A" },
            onoff(SUPPRESS.load(Ordering::Relaxed)),
            onoff(GUARD_FOREGROUND.load(Ordering::Relaxed)),
        ));
        log_accessibility();
        log("spike: hover the item and press the trigger once to capture its position");
    } else {
        ARMED.store(false, Ordering::Relaxed);
        log(&format!(
            "──── spike disarmed after {} roll(s) ────",
            ROLLS.load(Ordering::Relaxed)
        ));
    }
    Ok(())
}

/// Drop the captured position so the next press recaptures it.
pub fn forget_position() {
    POS_SET.store(false, Ordering::Relaxed);
    log("spike: forgot the captured item position — the next press will recapture it");
}

/// Write a line into the same log from the frontend's side of the seam.
pub fn note(line: &str) {
    log(&format!("spike: note — {line}"));
}

/// Everything the panel shows.
pub fn status() -> SpikeStatus {
    let trigger_vk = TRIGGER_VK.load(Ordering::Relaxed);
    let last_key_vk = LAST_KEY_VK.load(Ordering::Relaxed);
    SpikeStatus {
        hook_installed: lock(&HOOK).is_some(),
        armed: ARMED.load(Ordering::Acquire),
        learning: LEARNING.load(Ordering::Relaxed),
        suppress: SUPPRESS.load(Ordering::Relaxed),
        release_shift: RELEASE_SHIFT.load(Ordering::Relaxed),
        guard_foreground: GUARD_FOREGROUND.load(Ordering::Relaxed),
        trigger_vk,
        trigger_name: describe_vk(trigger_vk),
        last_key_vk,
        last_key_name: describe_vk(last_key_vk),
        keys_seen: KEYS_SEEN.load(Ordering::Relaxed),
        position: POS_SET.load(Ordering::Acquire).then(|| {
            (
                POS_X.load(Ordering::Relaxed),
                POS_Y.load(Ordering::Relaxed),
            )
        }),
        rolls: ROLLS.load(Ordering::Relaxed),
        max_rolls: MAX_ROLLS.load(Ordering::Relaxed),
        copy_delay_ms: COPY_DELAY_MS.load(Ordering::Relaxed),
        read_timeout_ms: READ_TIMEOUT_MS.load(Ordering::Relaxed),
        tolerance_px: TOLERANCE_PX.load(Ordering::Relaxed),
        presses: PRESS_SEQ.load(Ordering::Relaxed),
        shift_down: key_down(VK_SHIFT.0 as i32),
        foreground: foreground_window(),
        last_roll: lock(&LAST_ROLL).clone(),
    }
}

// ---------------------------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------------------------

/// Busy-wait for `duration`. Used instead of `sleep` wherever the number is measured or matters:
/// Windows rounds `sleep` up to the scheduler tick, which is ~15 ms — the same order as the
/// clipboard latency being measured.
fn spin_for(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    let started = Instant::now();
    while started.elapsed() < duration {
        std::hint::spin_loop();
    }
}

/// Take a lock, ignoring poisoning. A panic elsewhere must not make the spike unusable on the one
/// machine that can run it.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// `"Scroll Lock (0x91)"`. Covers the keys plausible as a trigger on a keyboard with no F-row;
/// anything else still reads back as a code the human can act on.
fn describe_vk(vk: u32) -> String {
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
        0x60..=0x69 => {
            return format!("Numpad {} (0x{vk:02X})", vk - 0x60);
        }
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
