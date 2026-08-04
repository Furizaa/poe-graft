# poe-graft

Over-roll protection for Path of Exile 1 crafting. You hold Shift with Orbs of Alteration in your
inventory and tap a trigger key once per roll; the app injects one Shift+left-click and one
`Ctrl+C`, parses the result, and the moment the mod you want lands in tier it **refuses the next
press** rather than acting on your behalf.

Planning lives on the issue tracker:
[Map: poe-graft — over-roll protection for PoE 1 crafting](https://github.com/Furizaa/poe-graft/issues/1).
The stack and the Rust/TypeScript seam are settled in
[ADR 0001](docs/adr/0001-stack-and-seam.md); read it before moving anything across the boundary.

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

Warm round trip is roughly 5–7 minutes. Any change to `Cargo.lock` busts the Rust cache and costs
12–20 minutes instead, so **batch dependency changes** rather than interleaving them with work you
want to see on Windows.

The workflow file's header comment lists the specific things that silently break the updater. Read
it before editing that file. Background and the empirical work behind it:
`docs/research/ci-and-auto-update.md` on the `research/ci-and-auto-update` branch.

## Diagnostics

The app shows its version, commit, and a link to the Actions run that built it, and appends a crude
log (including every updater event) to a file it will reveal in Explorer for you. Deliberately
minimal — the real diagnostics surface is
[Decide how to debug on a Windows box with no dev environment](https://github.com/Furizaa/poe-graft/issues/11).

| | Path |
| --- | --- |
| Log (Windows) | `%APPDATA%\com.furizaa.poegraft\logs\poe-graft.log` |
| Log (macOS) | `~/Library/Logs/com.furizaa.poegraft/poe-graft.log` |
| Install (Windows) | per-user, no UAC — NSIS `currentUser` |

## Compliance

The app performs **exactly one injected Shift+left-click and one `Ctrl+C` per trigger press you
physically make**, and nothing otherwise. It never runs on a timer, never repeats without a fresh
press, never moves the cursor, and never arms, cancels, or picks anything up. On a hit it refuses
the next press rather than reacting. See the map's Notes and
[Research: where GGG's line actually sits](https://github.com/Furizaa/poe-graft/issues/14).
