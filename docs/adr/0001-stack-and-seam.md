# ADR 0001 — Tauri v2, and where the Rust/TypeScript seam sits

- **Status:** Accepted
- **Date:** 2026-08-04
- **Deciding ticket:** [Decide the stack: Tauri, Electron, or .NET](https://github.com/Furizaa/poe-graft/issues/6)
- **Feeders:** [#2 click suppression](https://github.com/Furizaa/poe-graft/issues/2),
  [#3 clipboard contract](https://github.com/Furizaa/poe-graft/issues/3),
  [#5 builds and auto-update](https://github.com/Furizaa/poe-graft/issues/5),
  [#13 on-device probe](https://github.com/Furizaa/poe-graft/issues/13),
  [#15 the pivot](https://github.com/Furizaa/poe-graft/issues/15)

## Context

poe-graft prevents over-rolling while spamming Orbs of Alteration in Path of Exile 1. The human holds
Shift with orbs in the inventory and taps a trigger key once per roll; the app injects **one**
Shift+left-click and one `Ctrl+C`, parses the result, and on a hit **refuses the next press**.

Four constraints drive this decision, and they are not negotiable:

1. **The app is an actor, not a gate.** [#13](https://github.com/Furizaa/poe-graft/issues/13) proved PoE reads
   mouse buttons via Raw Input, so a physical click can never be suppressed. The app therefore *injects*
   input, which puts three Win32 APIs in the hot path — `SetWindowsHookExW(WH_KEYBOARD_LL)` for the trigger
   key, `SendInput` for the click, and clipboard poison + read for the result.
2. **Development happens on macOS**, where none of that integration exists. The decision logic has to be
   testable on the Mac; the Windows plumbing is only real on the gaming PC.
3. **The release pipeline is the only Windows test loop.** There is no dev environment on the gaming PC, so
   `merge → GitHub Actions → the installed app auto-updates` is the inner loop. Build time is a
   development-velocity cost, not release plumbing.
4. **The hook callback has hard rules.** Every keyboard event on the desktop waits inside it. The budget is
   300 ms hard (`LowLevelHooksTimeout`), and on the 11th overrun Windows **silently uninstalls the hook with
   no way for the app to detect it** — the app keeps believing it is armed while clicks flow freely. It fails
   **open**, which is the worst possible direction for this app.

## Decision

### Tauri v2 — TypeScript + React frontend, Rust core

### Layout

```
poe-graft/
  Cargo.toml                  workspace
  crates/core/                no Win32, no Tauri, near-zero dependencies
                              state machine · item parser · hit test · mod model
                              trait Platform { hook, inject, clipboard }
  crates/win32/               #[cfg(windows)] · impl Platform against `windows` 0.62
  src-tauri/                  thin: wires a Platform impl into core, exposes commands/events
  src/                        React + Vite + TypeScript
```

### The seam

Rust owns the entire roll cycle **and** the hit test. The contract across the webview boundary is
deliberately tiny:

| Direction | Payload |
| --- | --- |
| TS → Rust | `{ targetModGroup, maxTier, triggerKey, itemPosition }` |
| Rust → TS | `{ rollCount, lastRoll, state }` |

TypeScript owns mod selection, target configuration, the session panel and feedback. It holds **no domain
logic** — in particular it does not decide what counts as a hit.

The hit test stays in Rust because it is both the most safety-critical computation in the app and the most
trivial (`generation == "prefix" && tier <= N`, read straight off the annotated clipboard format), and
because it sits immediately next to the parser. Moving it up would split the domain model across two
languages to save writing about a hundred lines of the easiest Rust in the project.

> **Correction, 2026-08-04** — [ADR 0002](./0002-roll-cycle-and-hit-latch.md) supersedes the details here,
> though not the decision. **The hit test is not `generation == "prefix" && tier <= N`**: the annotated
> format it reads that off is a client setting, and its mod name varies per tier, so tier is derived from
> the rolled numbers instead and the annotation is only logged as a cross-check. **`itemPosition` is not in
> the TS → Rust payload** either — the app captures the Anchor itself, from the cursor, on the first press
> of a session. The seam's position and the "TypeScript holds no domain logic" rule are unchanged.

### Fail-closed on sequencing

This is what makes the seam safe regardless of where it sits: **the native side refuses to inject until a
verdict for the previous roll has arrived.** A slow or missing verdict throttles the roll rate; it never
over-rolls the jewel. This is affordable because reads were measured at **15–32 ms** over 113 real captures
([#16](https://github.com/Furizaa/poe-graft/issues/16)) while press-to-press is human-paced, and because
**~1.8% of reads time out** and must be treated as "do not act" anyway.

### Input layer is hand-rolled against `windows` 0.62

A dedicated thread installs `WH_KEYBOARD_LL` and runs a real `while GetMessageW(..) > 0` loop. Latch state is
an `AtomicU8`. The callback does one relaxed atomic load and a `wParam` comparison, then returns — no
allocation, no locks, no I/O, no third-party code.

## Consequences

### Good

- **The seam is a compile-time boundary, not a convention.** `core` cannot name a Win32 symbol, so the
  decision logic cannot drift into platform code without the compiler objecting.
- **Mac tests are fast and real.** `cargo test -p poe-graft-core` runs in seconds and never builds Tauri's
  dependency tree, replaying the 113 captured clipboard fixtures through the actual parser and hit test.

  > **Correction, 2026-08-04** — this was aspirational when written, and is now true, but the count was
  > wrong in a way worth recording.
  >
  > The fixtures landed with [#19](https://github.com/Furizaa/poe-graft/issues/19), in
  > `crates/core/tests/fixtures/captures/`, and `cargo test -p poe-graft-core` replays every one of them
  > through the parser and the hit test in ~1s.
  >
  > **The "113 captures" are one distinct Item Text repeated 113 times.** The AutoHotkey probe on
  > [#16](https://github.com/Furizaa/poe-graft/issues/16) copied the same jewel without ever rolling it, so
  > as fixtures the set is worth one file. The diversity comes from the 47 captured by our own code during
  > [#17](https://github.com/Furizaa/poe-graft/issues/17) — 41 distinct texts over 33 Mod Groups and every
  > tier from 1 to 6. Anyone sizing test confidence by "113 real captures" is off by two orders of
  > magnitude; `crates/core/tests/fixtures/README.md` states what each set does and does not cover.
  >
  > Since [ADR 0002](./0002-roll-cycle-and-hit-latch.md) they replay through the whole cycle, not only the
  > parser — the sequence needed for that is `captures/spike-17/manifest.json`, and spending it is
  > [#20](https://github.com/Furizaa/poe-graft/issues/20).
- **Tauri's one weakness is largely defused by the layout.** The 12–20 min cold build fires on `Cargo.lock`
  churn; `core` has near-zero dependencies, so most logic edits never touch the lock file and most iteration
  never reaches CI at all.
- **No native forks and no prebuilds to maintain.** One first-party crate covers hook, injection and
  clipboard.
- **5–15 MB installer**, near-instant to download on the target, and a small footprint beside a running PoE
  client.
- The updater works with **zero auth** now the repo is public — `latest.json` is just a public URL.

### Bad, and accepted

- **Cold CI builds are 12–20 min** whenever `Cargo.lock` changes. Mitigation: batch dependency changes rather
  than interleaving them with feature work you want to test on Windows; `swatinem/rust-cache` keyed on
  `Cargo.lock` handles the rest. Warm builds are 5–7 min, level with Electron.

  > **Correction, 2026-08-04** — measured while resolving
  > [#17](https://github.com/Furizaa/poe-graft/issues/17), and this consequence was wrong twice over.
  >
  > **Both numbers are too high.** The first build ever, with no cache at all, was **6m38s**, not
  > 12–20 min. Typical builds are **~3m**, not 5–7. The public runner is 4 vCPU / 16 GB and the
  > dependency tree is small.
  >
  > **More importantly, the trigger is wrong.** A `Cargo.lock` change does *not* cause a cold build.
  > `swatinem/rust-cache` falls back to a **prefix restore-key** that excludes the lockfile hash, so a
  > lockfile change misses the exact key, restores the previous cache regardless (`full match: false`),
  > and recompiles only what actually changed. Adding two crates cost **2m55s** — faster than the
  > measured no-change build, i.e. inside the noise.
  >
  > What genuinely goes cold is losing the *prefix*: the first build, a cache eviction after 7 unused
  > days, or a **Rust toolchain bump**, since the toolchain hash is part of the restore-key.
  >
  > This does not change the decision — Tauri was chosen with a cost that turns out not to exist — but
  > it does retire "batch your dependency changes" as a real constraint. It is tidy practice, nothing
  > more. Anyone sequencing commits around the cliff is paying for a cliff that is not there.
- **Roughly 150 lines of `unsafe` Rust are owned outright**, in the one place where mistakes are least
  forgiving. This is deliberate — see the alternatives below, where every option still leaves you owning a
  fork or a patch, just in a worse language for the job.
- **Three crates plus a workspace file** is more ceremony than a single crate.
- Debugging the native layer on a machine with no debugger remains genuinely hard. That is
  [#11](https://github.com/Furizaa/poe-graft/issues/11)'s problem, not solved here.

## Alternatives considered

### .NET / C# — eliminated on dev-loop grounds

Fastest CI of the three (3–5 min, no cold cliff) and Velopack's delta updates are the best of the field. It
still loses outright: there is **no `dotnet` on the Mac**, and WPF/WinUI **cannot run on macOS at all**, so
the UI could not be iterated on the development machine. Only Avalonia keeps the Mac usable, which means
adopting both a new toolchain and a less-travelled UI stack to gain about two minutes of CI.

### Electron — rejected, and the pivot made it worse

Everything would stay in one language the author is fluent in, there is no cold-build cliff, and its Windows
updater is the most mature of the three. `robotjs` also came back from the dead — 0.7.0 in March 2026, 0.8.0
on 2026-07-24, now on `node-addon-api` (N-API, ABI-stable) — which makes the injection half a maintained
package rather than hand-written code.

It loses on the native surface it cannot avoid:

- **Suppression is structurally impossible in `uiohook-napi` as published.** Its `dispatch_proc` copies the
  event and hands it to JS via `napi_call_threadsafe_function(.., napi_tsfn_nonblocking)`, then returns
  without ever touching `event->reserved` — libuiohook's suppression channel. The decision is gone before JS
  sees it, and no JS-side code can change that. A ~10-line vendored C fork setting `event->reserved = 0x01`
  on a native atomic flag is the fix, and it is a fork you own forever.
- **`iohook` has exactly the right latch API** (`disableClickPropagation()`) but is NAN-based with prebuilds
  stopping at Electron 14 / Node 16, last published 2021-06-14. Dead.
- So Electron means **two native dependencies, one of them your own C fork**, versus one first-party Rust
  crate — while the author has Rust installed and no particular reason to prefer C.
- Secondary costs: an 80–250 MB artifact and a heavier runtime beside a game that wants RAM.

**One conditional caveat, recorded honestly:** the trigger key is planned as an *unbound* key with
suppression as belt-and-braces. If suppression is ever dropped as a requirement, Electron's native surface
collapses to just `robotjs` and the C fork disappears. That would not overturn this ADR on its own, but it is
the one assumption whose removal would meaningfully narrow the gap.

### Convenience Rust crates for the input layer — all rejected

Checked against crates.io on 2026-08-04, not against the research snapshot:

| Crate | Version | Why not |
| --- | --- | --- |
| `rdev` (`unstable_grab`) | 0.5.3, Jun 2023 | Genuinely suppresses, but the API is declared unstable by its own docs; current work means a git dependency; `static mut` callback with no synchronisation; `grab()` calls `GetMessageA` **exactly once**, so the first stray posted message stops the hook being serviced; and it force-installs a mouse hook we no longer want. |
| `inputbot` | 0.6.0, Aug 2023 | `blockable_bind` is the right *shape*, but the hook proc does `MOUSE_BINDS.lock().unwrap()` **inside the callback**. A held lock stalls the entire desktop's input; a poisoned mutex panics inside an `extern "system"` fn and aborts the process. |
| `mouce` | 0.3.0 | Observe-only by construction — its handler ends unconditionally in `CallNextHookEx`, and its callback type has no return value, so the API could not express suppression. |

Both usable options are three years stale, and their hazards sit precisely inside the 300 ms callback where
failure is silent and fails open. Hand-rolling ~150 lines against Microsoft's own bindings is the smaller
risk.

### Alternative seam positions — rejected

- **Seam at the raw clipboard text** (Rust emits item text, TS parses and hit-tests, pushes `latched` down).
  Minimises Rust and gives the parser vitest plus the 113 captures. Rejected: it splits the domain model
  across two languages and puts the single most safety-critical computation in the GC'd layer, to avoid
  writing the easiest Rust in the codebase.
- **Seam at the parsed roll** (Rust emits a structured roll, TS owns the hit test and latch policy). Same
  objection, smaller. Rejected for the same reason.

Note that the latch **flag** lives in native code under every option, because the hook callback is what
decides whether to inject. Only the question of who *computes* the verdict was ever open.

### Frontend alternatives

**Svelte 5** and **vanilla TS** were both considered. The UI's only non-trivial widget is a fuzzy-search
combobox over the 126 mod rows in `data/ghastly-eye-jewel.json`; React has that off the shelf (cmdk, Radix,
shadcn), which also means [#9](https://github.com/Furizaa/poe-graft/issues/9)'s prototype output drops into
the real app instead of being rewritten. Bundle size, Svelte's main advantage, is irrelevant for a local
single-user app.

## Not decided here

- The clipboard crate. Poison-and-poll freshness is [#7](https://github.com/Furizaa/poe-graft/issues/7)'s,
  and the protocol determines whether raw `GetClipboardSequenceNumber` is needed or a wrapper suffices.
- The trigger key itself, and the named states of the cycle — both [#7](https://github.com/Furizaa/poe-graft/issues/7).
- `CONTEXT.md` and the project's ubiquitous language, which [#7](https://github.com/Furizaa/poe-graft/issues/7)
  produces.
- CI workflow shape and updater wiring — [#10](https://github.com/Furizaa/poe-graft/issues/10).
