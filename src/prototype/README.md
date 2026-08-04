# PROTOTYPE — the craft-session window ([#9](https://github.com/Furizaa/poe-graft/issues/9))

> Four candidate craft windows plus today's one as a control, switchable via `?variant=`, hosted on the
> existing craft surface.

Throwaway. Lives only on `prototype/craft-ui` and **must not reach `main`** — `main` is what the gaming
PC auto-updates from, and every push touching `src/**` publishes a release the owner is offered.

## Run it

```
pnpm dev            # → http://localhost:1420  ← the better loop: real browser, URL bar, devtools
pnpm tauri dev      # needed only to judge variant `live`, which uses the real backend
```

Switch with the **arrows in the floating bar** or the **`←` / `→` keys**. The Tauri webview has no URL
bar, so those are the real mechanism; the search param is written with `history.replaceState` so a
reload keeps the variant.

`pnpm dev` works because `data.ts` falls back to reading `data/ghastly-eye-jewel.json` straight off disk
when `invoke()` is unavailable. Every variant except `live` therefore runs with the **real 66-group mod
pool** in a plain browser.

## The question

The owner used the shipped app on the gaming PC and said: **"the UI is currently pretty hard to use —
walls of text and debug stuff."** That is the whole brief, and it points somewhere specific:

- The window is a **diagnostic instrument**. `Craft.tsx` is four numbered steps of badges — keystrokes
  seen, Shift held, Sticky Keys, Filter Keys, foreground, presses, dropped mid-cycle, cycle/copy ms — and
  `App.tsx` wraps it in a "This build" definition list, an Updates panel, a "Platform seam" section with
  a *Read platform* button, and a raw `<pre>` log dump.
- **Most of the text is Rust's.** `Sighting` is 58 words. `Halted` is 55 words in one paragraph with a
  three-item checklist buried inside it. No layout fixes a paragraph.

So the variants own the **whole window**, not just the craft section — a fixed craft panel judged inside
an unchanged debug shell would be judged inside most of the complaint. `live` renders today's `App`
untouched for comparison.

## The variants

Each takes a different position on *what earns a place on screen*, and — because it is #9's sharpest
open question — a different position on **how the Tier Threshold is expressed**.

| | Position | Threshold as | Changes core's copy? |
| --- | --- | --- | --- |
| `live` | Today's window, untouched. The control. | tier dropdown | — |
| **A** Cockpit | Same information, re-ranked. One big state, one primary control, every diagnostic in one collapsed strip. | tier dropdown | **No** — core's `message` verbatim |
| **B** Two-phase | Setup and session are different screens; arming replaces the window. Debug becomes a preflight checklist that must go green to arm, then is unreachable. | slider, bands and odds live | Yes |
| **C** Glanceable | One centred card, enormous state word, two numbers, everything else behind one disclosure. | **minimum roll** (`at least 23`) | Yes |
| **D** Ledger | Thin sticky strip; core's log is the primary surface, newest first, one line per event. Premise from [#11](https://github.com/Furizaa/poe-graft/issues/11). | radio rows, per-tier odds | Yes |

**A is the load-bearing comparison.** It is the only variant that changes no copy, so it answers whether
#9 is a CSS ticket or a `crates/core` ticket. If A is enough, `proposedCopy.ts` gets thrown away.

## What is real and what is faked

| Real | Faked |
| --- | --- |
| The mod pool — 66 groups, tiers, bands, required ilvl | `CycleStatus` — `mock.ts` drives it, `supported` stays `false` |
| The odds arithmetic — matches `docs/research/mod-tier-data.md` exactly | The log lines — core's phrasing, invented sequence |
| Item Text — 3 captures copied out of `spike-17/` | Nothing can click. There is no hook and no clipboard. |
| **Core's message strings — verbatim from `cycle.rs`** | |

Core's copy is reproduced word for word on purpose. Paraphrasing the thing the owner complained about
would make every variant look better than the real window and prove nothing.

The chrome is labelled **SIMULATED** so a replay on the Mac is never mistaken for a device run.

## Files

```
Shell.tsx        hosts the variants (UI.md sub-shape A); real pool with file fallback
Switcher.tsx     the floating bar, ←/→, history.replaceState
Driver.tsx       force any state, break the environment, audition the sounds — chrome, not design
mock.ts          synthetic CycleStatus + log; core's real message strings
data.ts          the tier data and the odds arithmetic
fixtures.ts      three real Item Texts, copied out of crates/core/tests/fixtures/captures/spike-17/
proposedCopy.ts  ⚠ a proposal to change crates/core, written in TS so it can be judged
soundBench.ts    three candidate sound sets × three signals
variants/        A-cockpit, B-twophase, C-glance, D-ledger
prototype.css    all of it, `pv-`/`pb-` prefixed; deleting this dir removes the prototype
```

`main.tsx` mounts the shell behind `import.meta.env.DEV`, with the import **dynamic and inside the
branch** — so a production build emits no chunk for any of it. Verified: no variant, mock, sound bench,
stylesheet or copy of the 128 kB tier data appears in `dist/`.

## Not decided here

`UI.md`: the useful outcome is usually *"the header from B with the sidebar from C"*. Nothing is folded
into `src/Craft.tsx` until a decision is recorded on #9.
