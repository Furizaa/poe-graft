# Windows click suppression — mechanism, latency budget, and runtime candidates

Research for [issue #2](https://github.com/Furizaa/poe-graft/issues/2). Investigated 2026-08-04.

**Scope:** how a Windows userland app can prevent a physical left-mouse press from reaching Path of Exile,
what that costs in latency and risk, and which of Rust / Node+Electron / .NET can actually do it.

Every claim below is tied to either a Microsoft Learn page or the actual source of the library in question.
Where the primary sources do not answer a question, it is listed under
[Open questions requiring the Windows machine](#open-questions-requiring-the-windows-machine) rather than guessed at.

---

## Verdict

Target load: ~8 clicks/sec = one press+release pair every ~125 ms, i.e. ~16 button callbacks/sec, plus a
continuous stream of `WM_MOUSEMOVE` callbacks at the mouse's polling rate (up to 1000/sec).
The per-callback budget is **300 ms** hard, and realistically **< 1 ms** if the desktop is to stay usable.
This load is trivial for every candidate that can suppress at all — the question is purely *can it suppress*,
not *can it keep up*.

| Runtime / library | Can swallow a physical click at ~8/sec? | Basis |
| --- | --- | --- |
| **Rust — `windows` / `windows-sys` direct** | **Yes** | Full `SetWindowsHookExW` + `WH_MOUSE_LL` + `CallNextHookEx` bindings exist; you control the return value. Recommended path. |
| **Rust — `rdev` (`unstable_grab`)** | **Yes**, with real caveats | `return 1` on `None` verified in source. But: API is declared unstable, published crate is 0.5.3 (Jun 2023), one-shot `GetMessageA` pump, `static mut` callback, forces a keyboard hook you don't want. |
| **Rust — `inputbot`** | **Yes** | `Bind::Block` / `Bind::Blockable` return `LRESULT(1)`. But the hook proc takes a `Mutex` + `.unwrap()` inside the callback — panic/latency hazard. Last publish Aug 2023. |
| **Rust — `mouce`** | **No** | Windows hook proc unconditionally ends in `CallNextHookEx`. Observe-only by construction. |
| **Node/Electron — `uiohook-napi`** | **No as published. Yes as a small fork.** | `dispatch_proc` copies the event and hands it to JS via `napi_call_threadsafe_function(..., napi_tsfn_nonblocking)`, then returns without ever touching `event->reserved`. The decision is gone before JS sees it. The underlying libuiohook *does* support suppression. |
| **Node/Electron — `iohook`** | Mechanically yes, practically **no** | Has exactly the latch API needed (`disableClickPropagation()`). But NAN-based, last npm publish 2021-06-14, prebuilds stop at Electron 14 / Node 16. Dead for a modern Electron app. |
| **Node/Electron — `node-global-key-listener`** | **No** | Keyboard only, no mouse events at all; repo archived read-only since 2024-07-19. Eliminated outright. |
| **Node/Electron — hand-rolled N-API addon** | **Yes** | Same as the Rust path, wrapped. Suppression decision must live in native code (an atomic flag), never in JS. |
| **.NET — `SharpHook` (`SimpleGlobalHook`)** | **Yes** | `SuppressEvent = true`, documented Windows+macOS, must be set synchronously, only on `SimpleGlobalHook`. Actively maintained (7.1.3, 2026-07-08), NuGet ships prebuilt `uiohook.dll`. |
| **.NET — raw P/Invoke `SetWindowsHookEx`** | **Yes** | Same mechanism; MS explicitly warns the callback must be a static method so the GC can't move it. |
| **Raw Input (`WM_INPUT`)** | **No — cannot suppress anything** | Raw Input is a read path. There is no suppression primitive in the API. Microsoft recommends it *instead of* hooks precisely when you are **not** blocking. |
| **Interception kernel filter driver** | **Yes**, and it is the only option immune to the raw-input question — but disqualify it | Requires admin driver install; its `mouse.sys` is flagged as forbidden by FACEIT and EasyAntiCheat, with vendors telling users to uninstall it. Do not ship this. |

**The one thing that could still invalidate all of the "Yes" rows:** it is *not* established by primary
documentation that suppressing at `WH_MOUSE_LL` also removes the event from a game's **Raw Input** stream.
If Path of Exile reads mouse buttons via `WM_INPUT` rather than legacy `WM_LBUTTONDOWN`, userland hook
suppression may not stop the click at all. This is the single highest-value on-device experiment and it must
be run before any implementation ticket is written. See
[Open question 1](#1-does-suppressing-at-wh_mouse_ll-also-remove-the-event-from-a-games-raw-input-stream).

---

## 1. The mechanism

### `SetWindowsHookEx(WH_MOUSE_LL)` is the only userland mechanism

[`SetWindowsHookExW`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)
installs a hook procedure into a hook chain. `WH_MOUSE_LL` (value `14`) installs a
[`LowLevelMouseProc`](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc).
Per the same page's scope table, `WH_MOUSE_LL` is **global only** — it cannot be scoped to one thread, so
it necessarily sees the whole desktop's mouse input.

The interception point is stated precisely:

> The system calls this function every time a new mouse input event is **about to be posted into a thread
> input queue**.
> — [LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)

And the suppression primitive:

> If the hook procedure processed the message, it may **return a nonzero value to prevent the system from
> passing the message to the rest of the hook chain or the target window procedure**.
> — [LowLevelMouseProc, Returns](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)

[Hooks Overview](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-hooks) confirms the general
model: "A hook procedure can act on each event it receives, and then **modify or discard** the event", and
that some hook types "can modify messages or stop their progress through the chain, preventing them from
reaching the next hook procedure or the destination window."

So: **returning nonzero from the `WH_MOUSE_LL` callback for `wParam == WM_LBUTTONDOWN` means no thread input
queue ever receives that press.** The foreground application does not get a `WM_LBUTTONDOWN`, does not get a
`WM_LBUTTONUP` (if you also swallow that), and — critically for a game — `GetAsyncKeyState(VK_LBUTTON)`
should also not report the press, because the button state was never committed to the input system. *(That
last inference is not spelled out in the docs; verify on device.)*

### Notably, the hook is **not** injected into the game

> However, the `WH_MOUSE_LL` hook is not injected into another process. Instead, the context switches back to
> the process that installed the hook and it is called in its original context. Then the context switches
> back to the application that generated the event.
> — [LowLevelMouseProc, Remarks](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)

This is why `WH_MOUSE_LL` needs no DLL, no injection, and no separate 32-bit build. It also means the whole
desktop's mouse input is routed through *our process* on every event — the source of both the latency budget
and the soft-lock risk.

### Raw Input is not an alternative — it is read-only

[Raw Input Overview](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-raw-input) describes only
registration (`RegisterRawInputDevices`) and reading (`GetRawInputData`, `GetRawInputBuffer`). There is no
filtering, consuming, or veto primitive anywhere in the API surface. Microsoft's own recommendation makes the
distinction explicit — Raw Input is what you use when you are *observing*:

> In most cases where the application needs to use low level hooks, it should **monitor raw input instead**.
> This is because raw input can asynchronously monitor mouse and keyboard messages that are targeted for
> other threads more effectively than low level hooks can.
> — [LowLevelMouseProc, Remarks](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)

And a Microsoft support post makes the "only if you're not blocking" caveat explicit:

> If you are monitoring keystrokes (**and not trying to block them**), you can get the keyboard input via Raw
> Input.
> — [Global hooks getting lost on Windows 7](https://learn.microsoft.com/en-us/archive/blogs/alejacma/global-hooks-getting-lost-on-windows-7)

**For poe-graft this matters twice over.** Raw Input is useless as a suppression mechanism, *and* Raw Input's
existence as a parallel delivery path is exactly what threatens the hook approach — see Open question 1.

### Telling physical clicks from our own synthesized input

[`MSLLHOOKSTRUCT.flags`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-msllhookstruct)
documents:

| Flag | Value | Meaning |
| --- | --- | --- |
| `LLMHF_INJECTED` | `0x00000001` | The event was injected (from any process) |
| `LLMHF_LOWER_IL_INJECTED` | `0x00000002` | Injected from a process at lower integrity level |

This is directly load-bearing for the map's TOS line ("every application of an Orb of Alteration is a physical
click by the human"): the gate can assert `!(flags & LLMHF_INJECTED)` before counting a click as a real roll,
and can refuse to ever *generate* a left click. `dwExtraInfo` is a second, self-chosen tag for input this app
injects itself (e.g. the `Ctrl+C` hover-copy) so the hook can recognise and ignore it.

---

## 2. Exact semantics of swallowing a click — design consequences

These follow from the mechanism rather than from a doc sentence, and each needs a decision in the
"design the click gate" ticket:

1. **Press and release are two separate hook callbacks.** `wParam` is one of `WM_LBUTTONDOWN` /
   `WM_LBUTTONUP` etc. — see the message list in
   [LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc). If the latch
   engages *between* a down and its up, swallowing the up leaves the game believing the button is still held.
   The safe rule: once a `WM_LBUTTONDOWN` has been let through, always let its matching `WM_LBUTTONUP` through;
   only start suppressing at the next down-edge. libuiohook's Windows implementation already suppresses press
   and release symmetrically via a single latch flag
   ([`grab_mouse_click`](https://github.com/wilix-team/iohook/blob/master/libuiohook/src/windows/input_hook.c)),
   which is the shape to copy.
2. **Nonzero return also stops the rest of the hook chain**, not just the target window. Microsoft warns:
   > Calling the `CallNextHookEx` function to chain to the next hook procedure is optional, but it is highly
   > recommended; otherwise, other applications that have installed hooks will not receive hook notifications
   > and may behave incorrectly as a result. You should call `CallNextHookEx` **unless you absolutely need to
   > prevent the notification from being seen by other applications**.
   > — [SetWindowsHookExW, Remarks](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)

   Suppression *is* that exception, but it means any other hook-based tool the user runs (AHK scripts, mouse
   vendor software, other PoE overlays) is blinded to the suppressed click. Acceptable, worth knowing.
3. **`SetWindowsHookEx` always installs at the head of the chain**, so a hook installed later by another
   application sits *ahead* of ours and can swallow the click before we see it
   ([Hooks Overview](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-hooks)). We are not
   guaranteed first refusal.
4. **Mouse moves flood the callback.** The same hook fires for `WM_MOUSEMOVE`. At a 1000 Hz polling rate that
   is ~1000 callbacks/sec, each of which contributes to the timeout budget. The callback must early-out on
   move events with essentially zero work.

---

## 3. Fullscreen DirectX, elevation, and UIPI

### Fullscreen-windowed vs fullscreen-exclusive

`WH_MOUSE_LL` is desktop-global and sits in the input path before any thread queue, so **the display mode of
the foreground window is not itself a reason for the hook not to fire**. There is no documented interaction
between hooks and DXGI presentation mode.

The real question is not the display mode but **which input API the game reads**, which the display mode only
correlates with. Many DirectX games switch to Raw Input (`RIDEV_NOLEGACY`) so that legacy `WM_MOUSEMOVE`
traffic doesn't clutter their queue — see
[Using Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input) and
[RAWINPUTDEVICE](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawinputdevice) for
the `RIDEV_NOLEGACY` semantics. Whether *Path of Exile 1* reads left-click via legacy messages or via
`WM_INPUT` is unverified. **Unverified — needs on-device check** (Open questions 1 and 2).

### Elevation and UIPI: a real blocker if the game runs elevated

Microsoft's own words on the boundary:

> User Interface Privilege Isolation (UIPI) implements restrictions in the Windows subsystem that prevent
> lower-privilege applications from sending messages or **installing hooks in higher-privilege processes**.
> — [User Account Control: Only elevate UIAccess applications that are installed in secure locations](https://learn.microsoft.com/en-us/windows/security/threat-protection/security-policy-settings/user-account-control-only-elevate-uiaccess-applications-that-are-installed-in-secure-locations)

The same page lists what a **UIAccess** process gains, and the third bullet is the decisive one:

> A process that's started with UIAccess rights has the following abilities:
> - Set the foreground window.
> - Drive any application window by using the `SendInput` function.
> - **Use read input for all integrity levels by using low-level hooks, raw input, `GetKeyState`,
>   `GetAsyncKeyState`, and `GetKeyboardInput`.**
> - Set journal hooks.
> - Use `AttachThreadInput` to attach a thread to a higher integrity input queue.

Reading that inversely: **without UIAccess (or matching integrity), a low-level hook does not read input for
higher integrity levels.** The archived UIPI post lists the blocked operations in the same spirit — a lower
privilege process cannot "use thread hooks to attach to a higher privilege process" or "use Journal hooks to
monitor a higher privilege process"
([What is UIPI on Vista](https://learn.microsoft.com/en-us/archive/blogs/vishalsi/what-is-user-interface-privilege-isolation-uipi-on-vista)).

Community corroboration of the concrete symptom, on Microsoft Q&A (answered by a community member, **not** a
Microsoft engineer — treat as a lead, not authority):

> your low level mouse hook will not be called until the foreground window changes to a lower integrity level
> process
> — [How can I tell in C# or VB.NET if a low-level hook is blocked by a high-integrity process?](https://learn.microsoft.com/en-us/answers/questions/2262996/how-can-i-tell-in-c-or-vb-net-if-a-low-level-hook)

**Practical consequences for poe-graft:**

- If PoE runs at the normal medium integrity level (the default via Steam or the standalone client) and
  poe-graft also runs at medium, UIPI is a non-issue. **No elevation needed.**
- If the user has ever set PoE to "Run as administrator", the hook goes deaf while the game has focus. The gate
  would silently fail open — clicks pass through, the item over-rolls, and nothing in the app is obviously
  broken. **The app must detect this and refuse to arm**, rather than pretend to be armed. That is a hard
  requirement for the safety/release-contract ticket.
- Detection recipe (from the same Q&A thread): `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` →
  `GetWindowThreadProcessId` → `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `OpenProcessToken(TOKEN_QUERY)`
  → `GetTokenInformation(TokenIntegrityLevel)`, comparing against `S-1-16-8192` (medium) / `S-1-16-12288` (high).
- **Going UIAccess is not a practical escape.** It requires a manifest `uiAccess="true"`, installation under
  `%ProgramFiles%` or `%WinDir%`, and — regardless of policy settings — "Windows enforces a PKI signature check
  on any interactive application that requests running with a UIAccess integrity level"
  ([same page](https://learn.microsoft.com/en-us/windows/security/threat-protection/security-policy-settings/user-account-control-only-elevate-uiaccess-applications-that-are-installed-in-secure-locations)).
  That means a real code-signing certificate rooted in Trusted Root CAs. Not viable for this project; note it
  as a constraint in the packaging ticket rather than a plan.
- **The UAC secure desktop is out of reach and that is a feature.** `SetWindowsHookEx` with `dwThreadId == 0`
  associates the hook with "all existing threads running in the **same desktop** as the calling thread"
  ([SetWindowsHookExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)).
  A UAC consent prompt runs on a separate desktop, so the latch cannot block clicks there — which is a useful
  guaranteed escape hatch for the soft-lock story.

---

## 4. The latency budget

### The hook runs on *our* message loop, and stalls the whole desktop while it does

> This hook is called in the context of the thread that installed it. **The call is made by sending a message
> to the thread that installed the hook. Therefore, the thread that installed the hook must have a message
> loop.**
> — [LowLevelMouseProc, Remarks](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)

Combine that with "about to be posted into a thread input queue": **every mouse event on the desktop is held
in place while our callback runs.** A slow callback is not just our problem; it is system-wide mouse lag.

### `LowLevelHooksTimeout`

> The hook procedure should process a message in less time than the data entry specified in the
> **LowLevelHooksTimeout** value in the following registry key:
>
> `HKEY_CURRENT_USER\Control Panel\Desktop`
>
> The value is in milliseconds. If the hook procedure times out, the system passes the message to the next
> hook. However, **on Windows 7 and later, the hook is silently removed without being called. There is no way
> for the application to know whether the hook is removed.**
>
> **Windows 10 version 1709 and later** The maximum timeout value the system allows is 1000 milliseconds
> (1 second). The system will default to using a 1000 millisecond timeout if the **LowLevelHooksTimeout** value
> is set to a value larger than 1000.
> — [LowLevelMouseProc, Remarks](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)

The default value and the strike count come from a Microsoft support engineer's post:

> On Windows 7 we have to make sure that the callback function of the hook can return in **less than
> LowLevelHooksTimeout, which is 300 ms**. And we allow for the application to be **timed out 10 times** when
> processing the hook callback message. If it times out an **11th time, Windows will unhook the application
> from the hook chain**. This is a **by design** feature and it was added in Win7 RTM.
> — [Global hooks getting lost on Windows 7](https://learn.microsoft.com/en-us/archive/blogs/alejacma/global-hooks-getting-lost-on-windows-7)

That post is from 2010 and covers Windows 7. The 300 ms default and the 10-strike rule should be treated as
"very likely still true on Windows 11" but confirmed on device by reading the registry value
(it is often absent, meaning the default applies). See Open question 4.

### The budget in practice

- **Hard ceiling per callback:** 300 ms (assuming default), and only ~10 breaches are forgiven before the hook
  vanishes silently.
- **Practical target:** < 1 ms. The callback must not allocate, must not lock anything another thread can hold,
  must not touch the clipboard, must not call into a garbage-collected runtime, and must not do I/O.
- **8 clicks/sec is nothing.** ~16 button callbacks/sec is three orders of magnitude below the ceiling. The
  latency risk is not throughput — it is a single pathological pause (GC, page fault, disk stall, a lock held
  by a busy thread) landing inside the callback.
- **Microsoft prescribes the exact architecture:**
  > If the application must use low level hooks, it should run the hooks on a **dedicated thread** that
  > **passes the work off to a worker thread and then immediately returns**.
  > — [LowLevelMouseProc, Remarks](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)

  For poe-graft that resolves cleanly, because the *decision* is always already made: the clipboard parse that
  determines "this jewel hit T1" happens after a previous roll. So the callback is a single relaxed atomic load
  of a `latched: bool` plus a `wParam` comparison. Everything else — parsing, sound, UI — goes to the worker.
- **Do not run the hook on Electron's main thread or on the .NET UI thread.** Either can be paused past 300 ms
  by GC or by JS work, at which point the hook is silently uninstalled and the gate fails open with no
  notification. The dedicated-thread rule is not optional here.
- libuiohook already does the right thing structurally: its worker creates a dedicated thread, sets
  `THREAD_PRIORITY_TIME_CRITICAL`, and runs `hook_run()` which owns a `GetMessage` loop
  ([`uiohook_worker.c`](https://github.com/SnosMe/uiohook-napi/blob/master/src/lib/uiohook_worker.c),
  [`libuiohook/src/windows/input_hook.c`](https://github.com/kwhat/libuiohook/blob/master/src/windows/input_hook.c)).
  Its own source comment states the constraint verbatim:
  > NOTE: The following callback executes on the same thread that `hook_run()` is called from. … some operating
  > systems may choose to disable your hook if it takes to long to process. If you need to do any extended
  > processing, please do so by copying the event to your own queued dispatch thread.

---

## 5. What happens when the owning process crashes or hangs

This is the soft-lock question. Establishing facts only; the safety design is a separate ticket.

### Case A — the process hangs (message loop stops pumping) while the latch is engaged

This is the dangerous case, and the docs describe it exactly. The hook is invoked by *sending a message* to
the installing thread; if that thread never processes messages, each mouse event waits out
`LowLevelHooksTimeout` before Windows gives up on it.

- **Every mouse event desktop-wide is delayed by up to ~300 ms.** The mouse becomes unusable, not just for the
  game.
- Windows then "silently removed" the hook after the 11th timeout
  ([LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc);
  [10-strike detail](https://learn.microsoft.com/en-us/archive/blogs/alejacma/global-hooks-getting-lost-on-windows-7)).
  **So a hang self-heals: after roughly 11 events × 300 ms ≈ 3 s of horrible lag, the OS uninstalls the hook and
  clicks flow again.** There is no permanent input lock from a hang.
- The flip side is that the same mechanism can uninstall the hook **while the app believes it is armed**.
  "There is no way for the application to know whether the hook is removed" — so the app cannot detect this
  from the hook API. It must either re-install periodically or verify liveness by watching that events are
  still arriving. That is a requirement for the release-contract ticket.
- Note that this is *also* why the hook must not be installed on a thread that ever blocks: the same 300 ms
  penalty applies to a normal-but-slow callback.

### Case B — the process crashes / is killed while the latch is engaged

- Microsoft says only:
  > Before terminating, an application must call the `UnhookWindowsHookEx` function to free system resources
  > associated with the hook.
  > — [SetWindowsHookExW, Remarks](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)

  I could not find a Microsoft statement that says explicitly "the system removes the hook when the owning
  process dies". **Unverified — needs on-device check** (Open question 3).
- The strong structural argument that it *is* safe: the hook procedure lives in our process's address space and
  the hook is only reachable by sending a message to a thread of that process. Once the process is gone there is
  no thread to send to, so at worst the timeout/strike mechanism from Case A applies and the hook is removed
  after ~3 s of lag. Either way there is **no plausible path to a permanent mouse lock from process death** —
  but this should be demonstrated on device, not assumed.
- `UnhookWindowsHookEx` has one relevant wrinkle even in the clean-shutdown path:
  > The hook procedure can be in the state of being called by another thread even after
  > `UnhookWindowsHookEx` returns.
  > — [UnhookWindowsHookEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex)

  So the callback and the data it reads must stay valid slightly past unhook. In Rust: don't drop the shared
  state immediately after unhooking; leak it or keep it in a `'static`.

### Case C — the user needs out, right now

Independent escape hatches that exist regardless of app state:

- **Ctrl+Alt+Del / the UAC consent prompt** run on a different desktop, and a global hook only covers "all
  existing threads running in the same desktop as the calling thread"
  ([SetWindowsHookExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)).
  The latch cannot follow the user there. This is the guaranteed floor on how bad a soft-lock can get.
- Killing the process (Case B).
- A release hotkey handled on the hook thread itself, so it works even if the rest of the app is wedged. Design
  note for the release-contract ticket: the hotkey must be evaluated in the hook callback path, not in JS/UI code.

---

## 6. Per-runtime findings

### 6.1 Rust — `windows` / `windows-sys` (recommended)

All required bindings exist in the `windows` crate under `Win32::UI::WindowsAndMessaging`:

```rust
pub unsafe fn SetWindowsHookExW(
    idhook: WINDOWS_HOOK_ID,
    lpfn: HOOKPROC,
    hmod: Option<HINSTANCE>,
    dwthreadid: u32,
) -> Result<HHOOK>

pub unsafe fn CallNextHookEx(
    hhk: Option<HHOOK>, ncode: i32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT
```

with `WH_MOUSE_LL`, `MSLLHOOKSTRUCT`, and `UnhookWindowsHookEx` all present
([SetWindowsHookExW](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.SetWindowsHookExW.html),
[CallNextHookEx](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.CallNextHookEx.html),
[MSLLHOOKSTRUCT](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/struct.MSLLHOOKSTRUCT.html)).
Feature gate: `Win32_UI_WindowsAndMessaging`. Because we return `LRESULT` ourselves, suppression is fully under
our control — nothing is abstracted away.

Shape of the correct implementation:

- A dedicated thread that calls `SetWindowsHookExW(WH_MOUSE_LL, ...)` and then runs a real
  `while GetMessageW(...) > 0 { ... }` loop, nothing else.
- A `static AtomicBool` (or `AtomicU8` for a small state machine) holding the latch.
- Callback body: `if code == HC_ACTION && wparam == WM_LBUTTONDOWN && LATCHED.load(Relaxed) { return LRESULT(1) }`
  else `CallNextHookEx(None, code, wparam, lparam)`.
- No panics across the FFI boundary: the callback is `extern "system"`, so a panic is UB/abort. Wrap in
  `catch_unwind` or, better, write a body that cannot panic.

This is the lowest-risk path and the one I'd recommend. It also composes with Tauri if the app ends up
Rust-hosted, and can be compiled as an N-API addon (`napi-rs`) if the app stays Electron.

### 6.2 Rust — `rdev` (`unstable_grab`): works, but read the source first

`rdev`'s Windows `grab` genuinely suppresses. From
[`src/windows/grab.rs`](https://github.com/Narsil/rdev/blob/main/src/windows/grab.rs):

```rust
if let Some(callback) = &mut *ptr {
    if callback(event).is_none() {
        return 1;
    }
}
...
CallNextHookEx(HOOK, code, param, lpdata)
```

and [`src/windows/common.rs`](https://github.com/Narsil/rdev/blob/main/src/windows/common.rs) installs both
`WH_KEYBOARD_LL` and `WH_MOUSE_LL` via `SetWindowsHookExA`. The documented contract is
"returning `None` ignores the event and returning the event let's it pass"
([`src/lib.rs`](https://github.com/Narsil/rdev/blob/main/src/lib.rs)). Verified identical in the
[published 0.5.3 source](https://docs.rs/crate/rdev/0.5.3/source/src/windows/grab.rs).

Caveats, all from the source:

- **Gated behind the `unstable_grab` feature**, and the crate says so: "the use of the word `unstable` here
  refers specifically to the fact that the `grab` API is unstable and subject to change"
  ([Cargo.toml](https://github.com/Narsil/rdev/blob/main/Cargo.toml), lib.rs docs).
- **Version drift:** crates.io has `rdev` **0.5.3, published 2023-06-26**; git `main` is an unpublished 0.6.0.
  Using `main` means a git dependency.
- **`grab()` calls `GetMessageA` exactly once**, not in a loop. Low-level hook callbacks are dispatched while
  the thread waits inside `GetMessage`, so it works — but the first ordinary posted message (a timer, a
  `WM_QUIT`, anything) makes `grab()` return `Ok(())` and the hook stops being serviced. Fragile for a
  long-running latch.
- **It installs a keyboard hook you did not ask for.** `grab` always calls both `set_key_hook` and
  `set_mouse_hook`. That doubles the surface exposed to the timeout/strike mechanism, and means every keystroke
  on the machine routes through our callback.
- **`static mut GLOBAL_CALLBACK`** with no synchronisation. Sound in practice only because the hook is always
  called on the installing thread; still, it is unsound Rust and can't be reviewed as safe.

Verdict: fine for a spike, not what I'd build the gate on. If you want a library, prefer forking the ~40 lines
you need over depending on this.

### 6.3 Rust — `inputbot`: suppression is a first-class feature, but the hook proc is risky

[`src/windows/mod.rs`](https://github.com/obv-mikhail/InputBot/blob/master/src/windows/mod.rs) `mouse_proc`:

```rust
Bind::Block(cb) => { let cb = Arc::clone(cb); spawn(move || cb()); return LRESULT(1); }
Bind::Blockable(cb) => { if let BlockInput::Block = cb() { return LRESULT(1); } }
```

so `MouseButton::LeftButton.blockable_bind(|| if latched { BlockInput::Block } else { BlockInput::DontBlock })`
is a genuine suppression API. `handle_input_events` installs `WH_MOUSE_LL` / `WH_KEYBOARD_LL` and runs a proper
`while … GetMessageW(…)` loop with a 100 ms `SetTimer` to keep it alive.

Problems, all inside the hook callback — i.e. inside the 300 ms budget:

- `MOUSE_BINDS.lock().unwrap()` — takes a `Mutex` **and** unwraps it. If another thread holds the lock, the
  callback blocks and the whole desktop's mouse stalls. If the mutex is poisoned (a panic anywhere else in the
  crate's callback machinery), `.unwrap()` panics *inside an `extern "system"` function* → abort.
- `set_hook` does `SetWindowsHookExW(...).unwrap()` — a UIPI/resource failure panics instead of surfacing an
  error you can act on.
- crates.io: **0.6.0, published 2023-08-17**.

Verdict: usable, and the `Blockable` design is exactly the right shape, but the mutex-in-callback pattern
conflicts with the latency rules above. Prefer lifting the pattern, not the dependency.

### 6.4 Rust — `mouce`: observe only

[`src/windows.rs`](https://github.com/emrebicer/mouce/blob/master/src/windows.rs) installs
`SetWindowsHookExA(WH_MOUSE_LL, …)` and its handler ends unconditionally with:

```rust
CallNextHookEx(HOOK, code, param, lpdata)
```

There is no path that returns nonzero. `hook()` takes a `Box<dyn Fn(&MouseEvent) + Send>` — a callback with no
return value, so the API could not express suppression even if the implementation wanted to.
**Eliminated for suppression.** (crates.io 0.3.0, 2025-05-17 — maintained, just not for this.)

### 6.5 Node / Electron — `uiohook-napi`: cannot suppress as published

This is the library Awakened PoE Trade uses, so it deserves a precise answer.

[`src/lib/addon.c`](https://github.com/SnosMe/uiohook-napi/blob/master/src/lib/addon.c):

```c
void dispatch_proc(uiohook_event* const event) {
  if (threadsafe_fn == NULL) return;

  uiohook_event* copied_event = malloc(sizeof(uiohook_event));
  memcpy(copied_event, event, sizeof(uiohook_event));
  if (copied_event->type == EVENT_MOUSE_DRAGGED) {
    copied_event->type = EVENT_MOUSE_MOVED;
  }

  napi_status status = napi_call_threadsafe_function(threadsafe_fn, copied_event, napi_tsfn_nonblocking);
  ...
}
```

The event is **copied**, queued to the JS thread **non-blocking**, and `dispatch_proc` returns. It never reads
or writes `event->reserved` — libuiohook's suppression channel. By the time JS sees the event, the native hook
callback has already returned and the click is long gone. **Suppression is structurally impossible in
`uiohook-napi` as published**, and no amount of JS-side code changes that.

For contrast, the underlying libuiohook *does* support suppression on Windows.
[`libuiohook/src/windows/input_hook.c`](https://github.com/kwhat/libuiohook/blob/master/src/windows/input_hook.c):

```c
LRESULT CALLBACK mouse_hook_event_proc(int nCode, WPARAM wParam, LPARAM lParam) {
    ...
    LRESULT hook_result = -1;
    if (nCode < 0 || event.reserved ^ 0x01) {
        hook_result = CallNextHookEx(mouse_event_hhook, nCode, wParam, lParam);
    } else {
        logger(LOG_LEVEL_DEBUG, "%s [%u]: Consuming the current event. (%li)\n", ...);
    }
    return hook_result;
}
```

`event.reserved = 0x01` → returns `-1` (nonzero) → the click is consumed. The capability is right there; only
`uiohook-napi`'s async bridge throws it away.

**Concrete fork path (small and well-scoped).** Add a native atomic latch and consult it in `dispatch_proc`
before dispatching:

```c
static atomic_bool suppress_left_click = false;   // set from JS via a new native binding, or via a
                                                  // SharedArrayBuffer byte for zero-call-overhead updates
void dispatch_proc(uiohook_event* const event) {
  if ((event->type == EVENT_MOUSE_PRESSED || event->type == EVENT_MOUSE_RELEASED)
      && event->data.mouse.button == MOUSE_BUTTON1
      && atomic_load(&suppress_left_click)) {
    event->reserved = 0x01;          // consumed by mouse_hook_event_proc, before dispatch returns
  }
  /* existing copy + tsfn dispatch, so JS still observes the event */
}
```

The decision stays on the hook thread; JS only ever *sets* the flag. That satisfies Microsoft's
dedicated-thread guidance and keeps the callback at nanoseconds. libuiohook's worker already runs on a
dedicated `THREAD_PRIORITY_TIME_CRITICAL` thread with its own `GetMessage` loop
([`uiohook_worker.c`](https://github.com/SnosMe/uiohook-napi/blob/master/src/lib/uiohook_worker.c)), so the
threading is already correct — this is a ~10-line change plus a binding.

Maintenance status is good: `uiohook-napi` **1.5.5, published 2026-03-21**, N-API (ABI-stable, no per-Electron
rebuild). If the app stays Electron, a vendored fork of this is the pragmatic answer.

### 6.6 Node / Electron — `iohook`: has the exact API needed, but is dead

`iohook` ships a vendored libuiohook fork with a **global latch** that is precisely the poe-graft primitive.
[`index.js`](https://github.com/wilix-team/iohook/blob/master/index.js):

```js
/** Disable mouse click propagation.
 *  The click event are captured and the event emitted but not propagated to the window. */
disableClickPropagation() { NodeHookAddon.grabMouseClick(true); }
enableClickPropagation()  { NodeHookAddon.grabMouseClick(false); }
```

and [`libuiohook/src/windows/input_hook.c`](https://github.com/wilix-team/iohook/blob/master/libuiohook/src/windows/input_hook.c):

```c
static unsigned short int grab_mouse_click_event = 0x00;
...
event.reserved = grab_mouse_click_event;   // in process_button_pressed AND process_button_released
...
UIOHOOK_API void grab_mouse_click(bool enabled) {
    if (enabled) { grab_mouse_click_event = 0x01; } else { grab_mouse_click_event = 0x00; }
}
```

Note it latches **press and release symmetrically** — the right semantics per §2.1.

Why it's still out:

- **Last npm publish 2021-06-14** (`iohook@0.9.3`); registry last modified 2022-06-19.
- **NAN-based, not N-API** — needs a prebuilt binary per ABI. Its `supportedTargets` stop at
  **Electron 14 / Node 16**. The environment here is Node 26. Building for a current Electron would mean
  maintaining the native build ourselves — at which point forking `uiohook-napi` (§6.5) is strictly better.
- 108 open issues, repo last pushed 2025-02-21, no releases since 2021.

Worth reading purely as the reference implementation of the latch.

### 6.7 Node / Electron — `node-global-key-listener`: eliminated

Keyboard only — no mouse events anywhere in the API. Windows implementation is an out-of-process key server
communicating over stdio. The repository was **archived read-only on 2024-07-19**, with the maintainers stating
the project has not been actively maintained for years
([repo](https://github.com/LaunchMenu/node-global-key-listener)). Rules itself out on both counts.

### 6.8 .NET / C# — `SharpHook`

`SharpHook` is a libuiohook wrapper and supports suppression explicitly.
[docs/articles/hooks.md](https://github.com/TolikPylypchuk/SharpHook/blob/master/docs/articles/hooks.md):

- Set `SuppressEvent = true` on the event args inside the handler to stop propagation.
- Must be assigned **synchronously** in the handler — asynchronous assignment does nothing.
- **Only supported on Windows and macOS** (not Linux).
- **Only works with `SimpleGlobalHook`.** `EventLoopGlobalHook` and `TaskPoolGlobalHook` cannot suppress
  "because event handlers are run on another thread" — the same synchronous constraint as everything else in
  this document.

Maturity is the best of any wrapper here: **NuGet 7.1.3, published 2026-07-08**; targets .NET 8/9/10,
.NET Framework 4.7.2+, .NET Standard 2.0; and the package **does** ship prebuilt natives —
`runtimes/win-x64/native/uiohook.dll`, `win-x86`, `win-arm64`, plus macOS/Linux (verified by inspecting the
`.nupkg` contents). The README's "not bundled" note applies to building from source, not to the NuGet package.

The catch that likely kills .NET for this project is environmental, not technical: **the map records no
`dotnet` on the macOS dev machine**, and the GC-pause hazard means the hook must be on a dedicated thread with
a handler that allocates nothing — doable but the least comfortable of the three runtimes for a hard
sub-millisecond callback.

### 6.9 .NET / C# — raw P/Invoke

Same mechanism, full control of the `LRESULT`. One Microsoft-documented .NET-specific trap:

> In .NET apps, you must ensure the callback is not moved around by the garbage collector (otherwise your app
> will crash with an `ExecutionEngineException`). One way to do this is by making the callback a static method
> of your class.
> — [SetWindowsHookExW, Remarks](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)

In practice: keep a rooted `static` delegate for the lifetime of the hook, and keep the handler allocation-free.

---

## 7. Alternatives to suppression

### 7.1 Interception (kernel-mode filter driver) — works, but do not ship it

[oblitum/Interception](https://github.com/oblitum/Interception) is a keyboard/mouse filter driver. Because it
sits below the OS input stack, it is the **only** approach that is unconditionally immune to the
raw-input question in Open question 1. It also fails every other test:

- **Requires an administrative driver install**: "driver installation requires execution inside a prompt with
  administrative rights" ([README](https://github.com/oblitum/Interception)). Built against WDK 7.1.0;
  repo last pushed **2021-08-09**.
- **Actively detected and blocked by anti-cheat.** A project issue reports FACEIT rejecting it:
  > Spent a solid hour trying to fix FACEIT which popped up an error saying that the `mouse.sys` driver was
  > forbidden. turns out that interception was the issue, and after uninstalling, that is fixed.
  > — [oblitum/Interception issue #170](https://github.com/oblitum/Interception/issues/170) (open since 2023-08-14)

  And a game vendor instructing users to uninstall it: iRacing's support article
  ["Please close Interception before starting the game"](https://support.iracing.com/support/solutions/articles/31000176764--please-close-interception-before-starting-the-game-)
  attributes the block to EasyAntiCheat and walks users through uninstalling the driver.
- Dual-licensed; commercial use requires a licence from the author.

Even though PoE 1 does not run a kernel anti-cheat, installing a globally-detected input filter driver on the
user's gaming PC to win one crafting QoL feature is disproportionate. **Recommend ruling this out** unless
Open question 1 comes back "hooks cannot suppress PoE's clicks", in which case it becomes the only remaining
mechanism and the whole feature needs re-scoping.

### 7.2 `BlockInput` — wrong tool

`BlockInput` blocks *all* keyboard and mouse input, which also blocks the release hotkey and the user's ability
to do anything else. It is a strictly worse soft-lock than the hook latch. Mentioned only to rule it out.

### 7.3 "Let the click through but make it harmless"

Worth exploring in the design ticket because it sidesteps both the raw-input risk *and* the soft-lock risk:
if no input is ever suppressed, there is nothing to unlock.

Candidate: on hit detection, `SetCursorPos` the cursor off the item onto empty inventory space, so the physical
click lands on nothing. Attractive because it never touches the input stream. Two unknowns:

- If PoE tracks its in-game cursor from Raw Input deltas rather than the OS cursor position, `SetCursorPos` may
  not move the in-game cursor at all — same root uncertainty as Open question 1.
- The race is tighter, not looser: the cursor must move before the click is dispatched, and a click already in
  flight cannot be recalled.

**Unverified — needs on-device check** (Open question 5). It is cheap to test and would be a materially safer
design if it works, so test it in the same session as Open question 1.

### 7.4 Wheel → left-click remap (from the map's "not yet specified")

Relevant here because the mechanism interacts: if left clicks arrive on the mouse *wheel* and are translated to
clicks by our own code, then "suppressing" becomes "simply not translating" — no hook return value needed, no
soft-lock possible, no raw-input exposure. Note that the wheel event itself would still need suppressing so the
game doesn't also scroll, and the TOS line in the map explicitly permits a 1:1 wheel→click remap while
forbidding synthesized currency clicks — a remap makes *every* click synthesized, so **this needs an explicit
TOS re-reading before it is treated as a viable escape.** Flagging the tension, not resolving it.

---

## 8. What existing PoE tools do

### Awakened PoE Trade — observes and synthesizes; never suppresses

Uses `uiohook-napi` (its author, SnosMe, wrote both).
[`main/src/shortcuts/Shortcuts.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/shortcuts/Shortcuts.ts):

```ts
import { uIOhook, UiohookKey, UiohookWheelEvent } from 'uiohook-napi'
...
uIOhook.on('keydown', (e) => { ... })
uIOhook.on('keyup',   (e) => { ... })
uIOhook.on('wheel',   (e) => { ... uIOhook.keyTap(UiohookKey.ArrowRight) ... })
...
uIOhook.keyTap(UiohookKey.C)    // the Ctrl+C hover-copy
```

The pattern is **observe → synthesize**, never intercept. Per §6.5 that is not a design choice so much as the
only thing `uiohook-napi` permits. Two takeaways:

- APT's hotkeys do not consume the original key — the game sees it too. Fine for APT's use case, fatal for a
  click gate.
- APT does implement a wheel→key remap (the `wheel` handler above), which is direct precedent for the map's
  wheel→click idea, and confirms the mechanism is available.

### PoE Overlay (Community Fork) — Electron 8 + `iohook` + `robotjs`

`package.json` pins `electron ^8.3.1`, `iohook ^0.6.5`, and a forked `robotjs`. So it is on the library that
*does* have `disableClickPropagation` (§6.6) — but on a version and an Electron generation far behind anything
poe-graft would ship. It is evidence the latch API has been used in this ecosystem, not a template to copy.

### Alteration-roller tools generally

The publicly visible open-source tools in this niche take the automation route rather than the suppression
route — e.g. [AwakenedAlterationSpam](https://github.com/w31w4ng/AwakenedAlterationSpam), a Python script that
captures item text, matches a regex, and **clicks itself** until the target hits, requiring windowed/borderless
mode. That is precisely the design the map rules out on TOS grounds ("the human clicks"). I found no
open-source tool implementing the "physical click, blocked on hit" design, so **poe-graft's approach has no
prior art to copy from** — which is worth knowing, because it means the mechanism has to be established
first-hand.

I could not identify the specific paywalled roller referenced in the ticket from public sources, so nothing
is claimed about how it works. **Unverified.** If its name is known, its behaviour under
"is the hook approach viable" is a useful existence proof and worth a follow-up.

---

## Open questions requiring the Windows machine

Ordered by how much they can change the plan. Questions 1–2 gate every implementation ticket.

### 1. Does suppressing at `WH_MOUSE_LL` also remove the event from a game's Raw Input stream?

**Why it matters:** if not, and PoE reads left-click via `WM_INPUT`, then *no userland approach works* and the
whole feature reduces to Interception (§7.1) or the harmless-click design (§7.3).

**What the primary sources establish:** legacy messages and `WM_INPUT` are two distinct delivery paths — that
is the entire point of `RIDEV_NOLEGACY`
([Using Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input),
[RAWINPUTDEVICE](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawinputdevice)).
`LowLevelMouseProc` is documented in terms of "about to be posted into a thread input queue"; whether the raw
input queue is upstream or downstream of the low-level hook is **not documented anywhere I could find**.

**Test:** write a tiny two-part probe. Part A registers for Raw Input (`RegisterRawInputDevices`, mouse TLC,
`RIDEV_INPUTSINK`) and logs every button transition from `WM_INPUT`, alongside `WM_LBUTTONDOWN` from the
window proc. Part B installs a `WH_MOUSE_LL` hook that swallows left clicks on a hotkey toggle. Run both, click,
and see which log lines disappear. Then repeat with a *second* process holding the Raw Input registration, to
rule out same-process effects.

### 2. Does the suppression actually stop a click inside Path of Exile — in both display modes?

**Test:** the app latched, PoE in the foreground, an Orb of Alteration on the cursor over a jewel. Click.
Did the item reroll? Run in fullscreen-windowed and in fullscreen-exclusive separately — these are different
answers, not one answer.

Also check whether PoE's own settings expose a raw-input or mouse-input option, and test both settings if so.
Confirm PoE's process integrity level is medium (`Process Explorer`, or the token check from §3) — if it is
high, expect the hook to be deaf while the game has focus, per §3.

### 3. Does the hook really disappear when the owning process is killed mid-latch?

**Why it matters:** it is the soft-lock floor, and Microsoft documents only the obligation to unhook, not the
cleanup behaviour (§5, Case B).

**Test:** latch, confirm clicks are dead, then `taskkill /F` the process. Measure how long until clicks work
again. Repeat with a deliberately hung message loop (a `Sleep(60000)` on the hook thread) and measure the same,
watching for the ~11 × 300 ms → silent unhook behaviour. Log whether the hook stops firing without any
notification, which is what the docs predict.

### 4. Confirm `LowLevelHooksTimeout` on the actual machine

Read `HKEY_CURRENT_USER\Control Panel\Desktop\LowLevelHooksTimeout`. It is commonly absent, meaning the default
applies. The 300 ms default and 10-strike rule come from a 2010 Windows 7 post
([source](https://learn.microsoft.com/en-us/archive/blogs/alejacma/global-hooks-getting-lost-on-windows-7));
the only currently-documented number is the Windows 10 1709+ **1000 ms ceiling**
([LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)). Measure the
real strike behaviour empirically as part of question 3 rather than trusting the 2010 figure.

### 5. Does `SetCursorPos` move PoE's in-game cursor?

Gates the harmless-click alternative (§7.3), which would be a materially safer design. **Test:** with PoE in
the foreground, call `SetCursorPos` to a known screen point and observe whether the in-game cursor follows.
Do it in both display modes. Cheap; run it in the same session as question 1.

### 6. Does swallowing only `WM_LBUTTONDOWN` leave the game with a stuck button?

Design detail from §2.1. **Test:** suppress the down but pass the up, and vice versa, and watch for a
stuck-button state in game (held-cursor behaviour, drag artifacts). Establishes whether the symmetric latch is
required or merely tidy.

---

## Recommendation

1. **Do not write an implementation ticket yet.** Open questions 1 and 2 can invalidate the entire approach,
   and they need the Windows box. Given the map's "no dev environment on the Windows box" constraint, the probe
   in question 1 should be built as a tiny standalone signed-nothing executable shipped through the same
   Actions → auto-update pipeline, or as a diagnostics mode in the app skeleton — which is an argument for
   building the pipeline and the diagnostics affordance *before* the gate.
2. **If the probes pass**, build the hook in **Rust against the `windows` crate**, on a dedicated thread, with
   an atomic latch and a callback that cannot allocate, lock, or panic. Expose it to whatever hosts the UI. If
   the app is Electron, that means an N-API addon (napi-rs), or equivalently a ~10-line fork of `uiohook-napi`
   (§6.5) — the fork is less code but inherits libuiohook.
3. **Design the release contract around two facts**: the hook can be silently uninstalled by the OS with no
   notification, and the hook goes deaf if the foreground process is elevated. Both mean the gate can fail
   *open* while the UI still says "armed". The app must actively verify liveness rather than assume it.
4. **Rule out** `mouce`, `node-global-key-listener`, `iohook`, and the Interception driver now, so no future
   session re-litigates them.
