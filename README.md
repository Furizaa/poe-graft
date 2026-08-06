# poe-graft

Over-roll protection for Path of Exile 1 crafting. You hold Shift with Orbs of Alteration in your
inventory and tap a trigger key once per roll; the app injects one Shift+left-click and one
`Ctrl+C`, parses the result, and the moment the mod you want lands in tier it **refuses the next
press** rather than acting on your behalf.

Planning lives on the issue tracker:
[Map: poe-graft — over-roll protection for PoE 1 crafting](https://github.com/Furizaa/poe-graft/issues/1).
The map is the route actually walked, with dated corrections where later tickets overturned earlier
findings — read it before trusting any single ticket in isolation.

## Where the decisions are written down

Start here rather than with the code. Every one of these is load-bearing, and several exist to record
a trap that cost a session to find.

| | |
| --- | --- |
| [`CONTEXT.md`](CONTEXT.md) | **The ubiquitous language.** Read first. Every capitalised term is used verbatim in code, tests, log lines, UI copy and tickets. |
| [ADR 0001](docs/adr/0001-stack-and-seam.md) | Tauri + Rust core; the compile-time seam. Read before moving anything across the TS/Rust boundary. |
| [ADR 0002](docs/adr/0002-roll-cycle-and-hit-latch.md) | The roll cycle, the seven states, the Latch, the Halt. **An Unknown Verdict costs a press, not an orb.** |
| [ADR 0003](docs/adr/0003-the-craft-window.md) | What the window looks like, the Mod Group picker, and the odds. |
| [`docs/spike-17-session.md`](docs/spike-17-session.md) | The on-device session that measured the mechanism. |

Research, one file per question, all with primary sources:

| | |
| --- | --- |
| [Windows click suppression](docs/research/windows-click-suppression.md) | Why a physical mouse click cannot be blocked in PoE — the finding that forced the pivot. |
| [The clipboard contract](docs/research/poe-clipboard-contract.md) | Poison-with-a-Sentinel, and why content can never prove freshness. |
| [GGG's automation line](docs/research/ggg-automation-policy.md) | The written "one action per keypress" rule this app is shaped around. |
| [Mod and tier data](docs/research/mod-tier-data.md) | RePoE provenance, and the real per-click hit rate. |
| [CI and auto-update](docs/research/ci-and-auto-update.md) | The Windows test loop, and the things that silently break the updater. |

## Layout

```
Cargo.toml        workspace — one shared target/ dir at the repo root
crates/core/      no Win32, no Tauri, near-zero dependencies. The Platform trait lives here
crates/win32/     #[cfg(windows)] · Platform against the `windows` crate
src-tauri/        thin: picks a Platform, opens the log, exposes commands
src/              React + Vite + TypeScript. Holds no domain logic
```

The seam is a **compile-time** boundary, not a convention: `crates/core` cannot name a Win32
symbol, so the decision logic cannot drift into platform code without the compiler objecting.

## Developing on macOS

The Mac is a development machine only — the app is not expected to work with a game here. Every
platform call reports "not supported", which is the stub doing its job.

```bash
pnpm install
pnpm tauri dev          # the window opens; the platform seam reports Unsupported
cargo test -p poe-graft-core   # the real loop: seconds, no Tauri in the tree
```

`cargo check -p poe-graft-win32 --target x86_64-pc-windows-msvc` type-checks the Windows crate
from the Mac without a linker. Use it before spending a CI build.

## Testing on Windows

There is no dev environment on the gaming PC, so **`merge → Actions → the installed app updates
itself` is the inner loop**, not release plumbing. Merging to `main` is the only manual step:
`.github/workflows/build-windows.yml` derives the version from `github.run_number`, builds an NSIS
installer, and publishes a signed release with `latest.json`.

Measured on the real loop, not estimated. **Merge → release is about 3 minutes**, and the full round
trip to a new version running on the gaming PC is under 4 minutes.

| Run | `Cargo.lock` | Merge → release |
| --- | --- | --- |
| First build ever, no cache at all | — | **6m38s** |
| Typical | unchanged | **3m14s** |
| Added two dependencies | **changed** | **2m55s** |

**A `Cargo.lock` change does not cost a cold build**, which is the opposite of what this project
assumed for its first three sessions. `swatinem/rust-cache` keys the exact cache on the lockfile
hash *and* falls back to a **prefix restore-key** that ignores it, so a lockfile change misses the
exact key, restores the previous cache anyway (`full match: false`), and recompiles only the crates
that genuinely changed. Adding `clipboard-win` and `error-code` cost nothing measurable.

What *is* genuinely cold is losing the prefix too: the first build, a cache eviction (GitHub expires
entries after 7 days unused), or a **Rust toolchain change** — the toolchain hash is part of the
restore-key, so a new `stable` release does buy the full rebuild.

Batching dependency changes is still tidy, but it is **not** load-bearing — do not contort a commit
sequence around it. See the dated correction in
[ADR 0001](docs/adr/0001-stack-and-seam.md#consequences).

The workflow file's header comment lists the specific things that silently break the updater. Read
it before editing that file. Background and the empirical work behind it:
[`docs/research/ci-and-auto-update.md`](docs/research/ci-and-auto-update.md).

## Diagnostics

The app shows its version, commit, and a link to the Actions run that built it, and appends a crude
log (including every updater event) to a file it will reveal in Explorer for you. Deliberately
minimal — the real diagnostics surface is
[Decide how to debug on a Windows box with no dev environment](https://github.com/Furizaa/poe-graft/issues/11).

| | Path |
| --- | --- |
| Log (Windows) | `%LOCALAPPDATA%\com.furizaa.poegraft\logs\poe-graft.log` |
| Log (macOS) | `~/Library/Logs/com.furizaa.poegraft/poe-graft.log` |
| Install (Windows) | `%LOCALAPPDATA%\poe-graft` — per-user, no UAC (NSIS `currentUser`) |

Both Windows paths are verified on the gaming PC, not inferred.

## The roll cycle

The whole cycle is a **pure state machine** in `crates/core/src/cycle.rs`: events in, commands out,
no clock and no Win32. `crates/win32/src/cycle.rs` executes the commands — the `WH_KEYBOARD_LL`
hook, `SendInput`, poison-and-poll — and reports what happened. It decides nothing.

That split is what makes the cycle testable on a machine with no game on it. `cargo test -p
poe-graft-core` replays the wrong-item `Halt`, a run of `Unknown` Verdicts, the stale Read that
parses perfectly and lies, the Latch, and all 81 roll records the on-device spike produced — in
under a tenth of a second. It also walks the reachable state space breadth-first to assert the one
invariant that matters: **no reachable path spends an Alteration without a fresh Miss for the item
in front of it.**

The design is [ADR 0002](docs/adr/0002-roll-cycle-and-hit-latch.md) and the vocabulary is
[`CONTEXT.md`](CONTEXT.md). Read both before changing either file; every capitalised term in them is
load-bearing in code, log lines and UI copy.

### The window

[ADR 0003](docs/adr/0003-the-craft-window.md) settles what the window looks like: one dominant state
with core's own sentence under it, the primary control beside the sentence that asks for it, and every
diagnostic in a single fold that opens itself on a Halt. It also covers the searchable Mod Group
picker — **a group is chosen, never an affix name**, because a group's name is per tier — and the odds
display, which is computed in `crates/core` and pinned to the research by
`crates/core/tests/odds.rs`. Read it before adding anything to the craft panel; the whole point of the
layout is what it refuses to show.

### The tier data is a bundled resource

`crates/core` embeds no data. `data/ghastly-eye-jewel.json` reaches the installed app through
`bundle.resources` in `src-tauri/tauri.conf.json` and is read once at startup. **If it fails to load
the app says so and refuses to arm** — there is no fallback tier table and there must never be one,
because guessing at the numbers is exactly how an over-roll gets through. The window shows which
file it actually read.

### Running a session on the gaming PC

1. Check the environment panel: Sticky Keys **off**, and `keystrokes seen` climbing when you press
   any key. Zero while the hook is installed means the hook is deaf
   ([#18](https://github.com/Furizaa/poe-graft/issues/18)), not that the window is wrong.
2. Pick the Target Mod and the Tier Threshold. Neither can change once armed.
3. Click **Arm**, then **click into Path of Exile** — arming is a mouse click in poe-graft's window,
   and the hook cannot hear the trigger while that window has focus.
4. Hold Shift with Orbs of Alteration in your **inventory**, hover the jewel, and tap `[`. The first
   press captures the Anchor and Reads; it does not Roll.
5. Keep tapping. A quiet blip means that press did not Roll — a Resync or a Refusal, and the window
   says which. On a Hit the app plays a loud sound and **refuses every further press** until you
   acknowledge it with the mouse.

The spike this replaced is gone; [docs/spike-17-session.md](docs/spike-17-session.md) is kept as the
record of the session that produced the measurements.

## Compliance

The app performs **exactly one injected Shift+left-click and one `Ctrl+C` per trigger press you
physically make**, and nothing otherwise. It never runs on a timer, never repeats without a fresh
press, never moves the cursor, and never arms, cancels, or picks anything up. On a hit it refuses
the next press rather than reacting. See the map's Notes and
[Research: where GGG's line actually sits](https://github.com/Furizaa/poe-graft/issues/14).
