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
| **A** ✅ **chosen** Cockpit | Same information, re-ranked. One big state, one primary control, every diagnostic in one collapsed strip. Mod group opens a **searchable modal**. | tier dropdown | **No** — core's `message` verbatim |
| **B** Two-phase | Setup and session are different screens; arming replaces the window. Debug becomes a preflight checklist that must go green to arm, then is unreachable. | slider, bands and odds live | Yes |
| **C** Glanceable | One centred card, enormous state word, two numbers, everything else behind one disclosure. | **minimum roll** (`at least 23`) | Yes |
| **D** Ledger | Thin sticky strip; core's log is the primary surface, newest first, one line per event. Premise from [#11](https://github.com/Furizaa/poe-graft/issues/11). | radio rows, per-tier odds | Yes |

**A is the load-bearing comparison.** It is the only variant that changes no copy, so it answers whether
#9 is a CSS ticket or a `crates/core` ticket. If A is enough, `proposedCopy.ts` gets thrown away.

## The decision (2026-08-05)

**A wins**, with one change the owner asked for: **the Mod Group control opens a searchable modal**
rather than a dropdown. B, C and D stay here as the primary source and are not maintained.

Two reasons given, and both point past this Base:

1. **Scale.** 66 groups is already a scroll; a Base with the full mod set behind it is several hundred,
   and a `<select>` with 300 options is not a choice. The driver has a **mod pool ×** control that
   duplicates the pool to 330 or 660 groups so this is tested rather than asserted — see below.
2. **Room for rich search and grouping by type**, which a dropdown has nowhere to put.

A also read as "the one ready to allow multiple mods". Worth noting that **multiple Target Mods is a
`crates/core` change, not a layout one** — `Target { group_id, tier_threshold }` is singular, and the
Hit rule for several targets is genuinely undecided (any of them, or all of them?). Not attempted here.

### `ModPicker.tsx` — what it does and why

- **Tokens are AND-matched, in any order**, against the group's rendered lines *and* every tier's affix
  name. So `minion phys`, `annealed` and `flaring` all find the one group.
- **An affix name may match a row but never label one.** A group's name is per tier, so `Flaring` (T1)
  and `Annealed` (T4) are one group; listing them separately is the trap
  [#4](https://github.com/Furizaa/poe-graft/issues/4) fell into. A name match shows as a chip —
  *matches "Annealed" at T4* — and the label stays the rendered line.
- **Mods an Orb of Alteration cannot roll are listed and disabled**, in their own greyed section, because
  a target that can never hit is a craft that can never end and an absent row looks like a broken search.
  Today that is `corrupted` (Vaal Orb) and `delve` (fossil). Six group ids appear in **both** pools —
  `AvoidIgnite`, `AvoidStun`, `ChanceToAvoidBleeding`, `ChanceToAvoidFreezeAndChill`,
  `ChanceToAvoidPoison`, `PercentDamageGoesToMana` — and those stay selectable, because they really are
  alteration mods too.
- **Keyboard**: search is focused on open, `↑`/`↓` move, `↵` chooses, `esc` closes.
- Picking clamps the Tier Threshold to the new group's worst tier — 40 of 66 groups have only a Tier 1,
  and Rust rejects a Target Mod naming a tier its group lacks.

### Measured, not assumed

Filter + full re-render per keystroke, dev build with StrictMode double-rendering (production is faster):

| pool | rows rendered | per keystroke |
| --- | --- | --- |
| 66 (real) | 66 | instant |
| 330 (×5) | 330 | 26–100 ms |
| 660 (×10) | 660 | 41–50 ms |

**No virtualization is needed at any plausible mod count.** Worth re-checking only if a Base ever pushes
past a couple of thousand groups.

### What the data cannot do yet

Grouping by **influence / eldritch** is not possible from `data/ghastly-eye-jewel.json` — there are zero
influence markers in it, because an Abyss Jewel cannot carry influence mods; those belong to equipment
bases. The categories that *do* exist are `corrupted`, `delve_prefix` and `delve_suffix`. Adding influence
and eldritch groupings means **extending the generator and the schema**, and then extending the seam:
`ModGroup` has no category field, and no `spawn_weight` either (see below).

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
Shell.tsx        hosts the variants (UI.md sub-shape A); real pool with file fallback; pool inflation
Switcher.tsx     the floating bar, ←/→, history.replaceState
ModPicker.tsx    ✅ the searchable Mod Group modal — the chosen picker
Driver.tsx       force any state, break the environment, stress the pool, audition the sounds
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

## Still to do

The decision is recorded above and on #9. **A + `ModPicker` has not yet been folded into
`src/Craft.tsx`** — that is a deliberate rewrite rather than a promotion of this code, and the push that
carries it to `main` publishes a release the gaming PC will offer to install, so it wants to happen when
the owner is happy with the picker rather than automatically.

Open, and each arguably its own ticket:

- **Multiple Target Mods** — a `crates/core` change; the Hit rule for several targets is undecided.
- **Categories for the picker** (influence, eldritch, …) — generator, schema and seam, in that order.
- **`spawn_weight` on the seam**, without which no shipped UI can display odds.
- **Shortening core's `Sighting` / `Halted` strings** — `proposedCopy.ts` is the proposal.
- **A permanent macOS simulator** — a real `CraftSession` behind the `cfg(not(windows))` stub.
