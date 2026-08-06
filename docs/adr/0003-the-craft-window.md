# ADR 0003 — The craft window: one dominant state, diagnostics folded, odds computed in Rust

- **Status:** Accepted
- **Date:** 2026-08-05
- **Deciding ticket:** [Prototype the mod selection and craft session UI](https://github.com/Furizaa/poe-graft/issues/9)
- **Feeders:** [#7 the roll cycle](https://github.com/Furizaa/poe-graft/issues/7),
  [#4 the tier data](https://github.com/Furizaa/poe-graft/issues/4),
  [#18 the deaf hook](https://github.com/Furizaa/poe-graft/issues/18),
  [#11 debugging without a dev environment](https://github.com/Furizaa/poe-graft/issues/11)
- **Vocabulary:** [`CONTEXT.md`](../../CONTEXT.md) — every capitalised term below is defined there
- **Prototype:** `prototype/craft-ui` — four candidate windows plus the sound bench, kept as the primary source

## Context

[ADR 0002](0002-roll-cycle-and-hit-latch.md) settled what the cycle *does*. What the window should look
like was left to #9, and the first version — shipped by
[#20](https://github.com/Furizaa/poe-graft/issues/20) so the acceptance test could happen — was
explicitly not a design: four numbered steps of badges, giving the hook counters, the accessibility
flags, the timings and the Target Mod picker equal weight.

The owner used it on the gaming PC and returned a verdict: **"the UI is currently pretty hard to use —
walls of text and debug stuff."** That sentence is this ADR's entire brief, and it pointed somewhere
specific:

- **The window was a diagnostic instrument.** `Craft.tsx` was the badge wall; `App.tsx` surrounded it
  with a "This build" definition list, an Updates panel, a "Platform seam" panel with a *Read platform*
  button, and a raw `<pre>` of the log. Together those were most of the complaint, which is why this
  ADR covers the whole window rather than only the craft panel.
- **Most of the text is Rust's.** `Sighting` is 58 words. `Halted` is 55 words in a single paragraph
  with a three-item checklist buried inside it. No layout fixes a paragraph — so part of the answer
  had to be a `crates/core` change, and establishing *which part* was the point of the prototype.

Four structurally different windows were built on `prototype/craft-ui`, each also taking a different
position on how the Tier Threshold is expressed, plus a sound bench. The comparison that mattered was
**A**, because it was the only variant that changed no copy: if A was enough, #9 was a layout ticket.

## Decision

### 1 · One dominant state, and the control beside the sentence that asks for it

Variant **A** wins. The shape:

- **The state is the biggest thing on screen**, with core's own `message` directly beneath it. The
  band's left edge carries the tone, so the state is readable from a second monitor without reading the
  word.
- **The primary control sits next to the sentence that asks for it** — Arm, Acknowledge the Hit, or
  Re-arm — never further down the page than the copy demanding it.
- **The target is a sentence** (*"Looking for … at Tier 1 or better — 23–26 to 33–39. About 1 in 272
  rolls."*), with the controls underneath rather than instead of it.
- **Every diagnostic is in one fold at the bottom.** Nothing is deleted: on a machine with no dev
  tools this is the entire diagnostic surface. It grows a red dot on the summary when something inside
  needs reading, and **opens itself on a Halt**.
- `App.tsx` gets the same treatment — build info, updater controls, the platform seam and the log tail
  are one fold. **A waiting or failed update is promoted back out**, because that is a decision the
  human has to make rather than a diagnostic they may want to read.

**No `message` string changed.** That is what made A the honest comparison, and it means this ADR
does *not* settle the copy question — see [Consequences](#consequences).

### 2 · The Mod Group is chosen in a searchable modal, never from a dropdown

A `<select>` of 66 groups is already a scroll, and a Base with the game's full mod set behind it is
several hundred, where a dropdown stops being a choice at all. So the Mod Group control opens a modal
with a search field, grouped sections and enough per row to tell two similar mods apart. The Tier
Threshold **stays a dropdown**: at most six options, ordered, which is the one case a native select is
good at.

**The rule the modal exists to enforce: a group is chosen, an affix name never is.** A group's display
name is per tier — the group behind *Minions deal # to # additional Physical Damage* is `Flaring` at T1
and `Annealed` at T4 — so a list offering both as entries offers the same mod twice and lies about
both. That is the trap [#4](https://github.com/Furizaa/poe-graft/issues/4) walked into.

Searching by name is still worth having, because that is how mods are talked about. The resolution:
**a name may match a row but never label one.** The label is always the rendered line; a name match is
reported as a chip — *matches "Annealed" at T4*. Both `flaring` and `annealed` find the one group.
Tokens are AND-matched in any order, against the lines and every tier's affix name.

Measured on the prototype at 330 and 660 groups: 26–100 ms and 41–50 ms per keystroke in a dev build
with StrictMode double-rendering. **No virtualization is needed at any plausible mod count.**

### 3 · The odds are computed in Rust, and their inputs are optional

#9 asked for the roll count and cumulative probability in place of the roll cap ADR 0002 removed.
Nothing could compute them: `crates/core` never parsed `spawn_weight` or `pool_totals_by_ilvl`.

It does now. `ModPool::odds(group_id, tier_threshold, ilvl)` returns the per-click rate, the
conditional, `1 in N` and the median.
[ADR 0001](0001-stack-and-seam.md) keeps domain logic behind the seam, so the frontend formats these
and does not derive them — with **one stated exception**: `1-(1-p)^n` for the cumulative figure, which
changes with every Roll and would otherwise cost an `invoke` per poll. It compounds a probability Rust
already established rather than deriving one.

**All three odds inputs are optional**, which is the load-bearing part. Odds are informational — no
Verdict, Refusal, Halt or Latch consults any of them — so a pool without weights must still load and
still assess Reads. Refusing to start over a number nothing depends on would make an unrelated feature
safety-critical. A missing weight yields **"cannot say"**, not **"impossible"**: those are different
claims and the window renders them differently.

The model, from `docs/research/mod-tier-data.md`: an Alteration reforges a magic item, which gets one
or two affixes at the file's `magic` weights (1:1) and holds at most one prefix and one suffix. With
two affixes the wanted generation is drawn for certain; with one it must first win a coin weighted by
the two slot totals. So
`p = P(two)·w/W_own + P(one)·w/(W_prefix+W_suffix)`.

### 4 · The window carries a jewel item level

A consequence of (3) that nobody had noticed: **the odds move with item level.** The figure recorded
across three sessions as 0.3591% is the ilvl ≥ 86 row; the same target on the ilvl 83 jewel the
fixtures actually contain is 0.3680% — 1 in 272. Before arming, the UI has to assume a level, so it
takes one (defaulting to 83, which is what T1 of the acceptance-test target requires). Mid-session it
need not assume anything: the parser already reads `Item Level:` off the Item Text.

### 5 · Sounds: the Chime set

The three synthesised WebAudio placeholders from ADR 0002's semantics are **kept as the answer** — a
rising sine arpeggio for the Hit, low square pulses for the Halt, a short quiet blip for "that press
did not Roll" — chosen by the owner from a bench of three candidate sets (Chime, a struck Bell, and a
deliberately unpleasant Alarm) on `prototype/craft-ui`.

One known weakness, recorded rather than fixed: **the blip at 1500 Hz sits inside the Hit's spectral
range** (880 → 1320 → 1760 Hz), so the two are distinguished only by length and volume, which is the
least reliable difference over game audio. Bell and Alarm both separated them by pitch direction
instead. If the blip is ever mistaken for a Hit in real use, that is the fix, and the bench is still on
the prototype branch.

## Consequences

- **`crates/core` gained an informational surface.** `Odds`, `PoolTotals`, `ModTier::spawn_weight` and
  `ModPool::totals_at`/`odds` are the first things in `core` that no decision consults.
  `crates/core/tests/odds.rs` pins them to `docs/research/mod-tier-data.md` row by row, because if the
  arithmetic drifts the window starts telling the human something the research does not say and
  nothing else would catch it.
- **One correction to the research doc.** Its ilvl 83 median of 188 is a rounding of 188.01; at 188
  Rolls the cumulative chance is 49.997%, so the first count that is genuinely more likely than not is
  **189**. `Odds::median_rolls` returns the ceiling and the doc now carries the footnote.
- **The copy question is still open, and is the strongest single finding of the prototype.** Rendering
  `Halted`'s buried checklist as three short questions beat the 55-word paragraph outright. Doing that
  for real is a `crates/core` change, because the window and the log must not disagree — the concrete
  proposal is `src/prototype/proposedCopy.ts` on `prototype/craft-ui`.
- **Core's copy still embeds raw group ids** — *"Armed for `MinionAddedPhysicalDamage` at Tier 1 or
  better"*. Debug identifiers in human-facing prose, and the ledger variant made the repetition
  obvious. Part of the same copy change.
- **The picker cannot group by influence or eldritch yet**, and this is a data limit rather than a UI
  one: there are zero influence markers in `data/ghastly-eye-jewel.json`, because an Abyss Jewel cannot
  carry influence mods. Reaching that means extending the generator, then the schema, then the seam —
  `ModGroup` has no category field. The prototype has a working version of the related affordance:
  mods an Orb of Alteration **cannot** roll (`corrupted`, `delve`) listed and greyed out, so an absent
  row explains itself instead of looking like a broken search. Deferred because every group in the
  picker today is alteration-reachable, so there is nothing yet to grey out. Note six group ids appear
  in **both** pools and must stay selectable.
- **The odds display is a point estimate with a caveat, not a range.** The community-derived
  affix-count split bounds the truth to **1 in 223 … 1 in 349**, so the strictly truthful rendering is
  that interval. Deliberately not shown: it is more words on screen, which is the thing this ADR
  exists to reduce. Recorded so the trade is not rediscovered as a bug.
- **A real bug was found and fixed on the way.** `enableSounds()` was called from exactly one place —
  the Arm button — and a webview may suspend an `AudioContext` while the game has focus and our window
  is in the background, which is every long Craft Session. Nothing resumed it, so **the Hit sound could
  go silent mid-craft** with no way back short of re-arming. `play()` now resumes before every sound.
- **Variants B, C and D are not maintained.** They live on `prototype/craft-ui` as the primary source,
  along with the sound bench, the mock status feed and `proposedCopy.ts`. That branch must not be
  merged: `main` is what the gaming PC auto-updates from.
