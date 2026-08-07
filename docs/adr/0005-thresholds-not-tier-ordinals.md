# ADR 0005 — A Member holds a Threshold, not a tier ordinal

- **Status:** Accepted
- **Date:** 2026-08-07
- **Deciding ticket:** [#39 Decide whether a Member holds a numeric threshold instead of a tier ordinal](https://github.com/Furizaa/poe-graft/issues/39)
- **Feeders:** [#23 the tier-overlap census](https://github.com/Furizaa/poe-graft/issues/23),
  [#22 the essence research](https://github.com/Furizaa/poe-graft/issues/22),
  [#21 the map](https://github.com/Furizaa/poe-graft/issues/21)
- **Vocabulary:** [`CONTEXT.md`](../../CONTEXT.md) — every capitalised term below is defined there
- **Research:** [`docs/research/tier-range-overlaps.md`](../research/tier-range-overlaps.md) — the census
  the decision rests on; the numbers matter more than the argument

> ADR 0004 is reserved for the craft window's own design
> ([#31](https://github.com/Furizaa/poe-graft/issues/31)), per the map's charting. This ADR took the
> next free number rather than the reservation.

## Context

Map [#21](https://github.com/Furizaa/poe-graft/issues/21) charted `Member = { group_id, tier_threshold }`
and a per-Member straddle rule: every matching tier satisfies the threshold → `Hit`, none → `Miss`, a
genuine straddle falls to the game's `(Tier: N)` annotation. [#23](https://github.com/Furizaa/poe-graft/issues/23)
was charted as the ticket that could falsify that, and mostly cleared it — 18 overlapping tier pairs
over 120 Base Groups, worst per-Roll ambiguity 6.5%, flasks essentially clean, and the acceptance craft
entirely clean because Deafening Essence of Zeal grants a flat 32% wedged between the normal 30% and 35%.

What it did not clear is the assumption underneath: **that a tier *ordinal* is a stable thing to store.**
Three findings, each independently sufficient:

- **Overlapping bands.** 18 distinct pairs, 8 Mod Groups, 71 of 120 Base Groups. Shallow but real, and
  guaranteed rather than hypothetical for essence crafts — the elemental avoidance ladders step in
  1-point increments across 2-wide bands, so every roll of a middle rank matches two tiers.
- **Tiers no annotation can order.** 17 pairs share a `required_ilvl`. The generator breaks those ties
  alphabetically by `mod_id`; GGG breaks them however GGG breaks them, and nothing in the export says.
- **Renumbering.** Folding essence mods into a ladder renumbers **241 normal tiers across 32
  (class, group) pairs** — including this map's own acceptance Member, where *30% increased Movement
  Speed* is Tier 2 without essence mods in the ladder and Tier 4 with. Essence-only mods print no
  `(Tier: N)` at all, so the app cannot even ask the game which numbering it uses.

Every one of those is an ambiguity about *which tier*. **None of them is an ambiguity about what the
numbers are** — the rolled numbers are printed on the item and are never in doubt.

The case against changing anything was that tiers are how the domain talks: `CONTEXT.md` made **Tier**
and **Tier Threshold** first-class, [ADR 0003](0003-the-craft-window.md)'s picker is built on them, the
odds research is expressed in them, and the owner asked for this map's crafts in them. That case is
answered rather than overridden — see decision 1.

## Decision

### 1 · A Member holds numbers; a Tier is how they are chosen

`Member = { group_id, thresholds }`. Picking "Tier 2" resolves that tier to a **Threshold** — a numeric
bound per rendered value — **once, when the Rule is built**. `assess()` compares numbers to numbers and
never derives a tier in order to decide a Verdict.

The tier survives everywhere it was already earning its place: in the picker, in the vocabulary, and in
the window's sentence. It stops being a thing the hit test reasons about. That is exactly the distinction
#23 drew when it observed that the code asks *"which tier is this?"* when the question it needs answered
is *"does this satisfy the Member?"*.

**This amends two charted decisions** — the rule model and the straddle rule. The straddle rule is not
sharpened, it is retired: there is no straddle to resolve when the comparison is arithmetic.

### 2 · The Threshold is one bound per *rendered value*, and its direction comes from the ladder

The bound is the picked tier's `display_min`, and it is keyed to **rendered placeholders, not to the stat
array**. Those are not the same thing: `ChanceToAvoidFreezeAndChill` carries two stats
(`base_avoid_chill_%` at 0/0 and `base_avoid_freeze_%` at 41-50) and renders one number.

A **floor** rather than the picked tier's band, and rather than membership in the set of acceptable
tiers' bands. The band-set reading is the smaller change — it is the straddle rule's first branch with
the annotation branch deleted — and it is wrong, for a reason the acceptance craft demonstrates:
`MovementVelocity` on boots is a flat-value ladder (10/15/20/25/30/35) and the essence value is a flat
32. Under a band set, "Tier 2 or better" is `{30}` ∪ `{35}`, and a legitimate 32% roll matches nothing,
yielding `NoTierMatched` and a Hit-via-anomaly. Under a floor, `32 >= 30` is a Hit with nothing to
explain. #23 found that pitching essence ranks *between* normal tiers is how GGG builds them, so this is
the common shape rather than a corner.

**Direction is derived per stat from the ladder**, at generation time:

| the stat's `display_min` across the ladder | reading | comparison |
| --- | --- | --- |
| falls as tiers worsen | higher is better | `value >= bound` |
| rises as tiers worsen | lower is better | `value <= bound` |
| never varies | carries no information | excluded from the Threshold |
| non-monotone in both directions | data fault | refused at load |

This needs no new data and no hand-maintained polarity table — the ladder already says it. It is here
because a drawback line (`#% less Duration`, shared by five flask immunity groups; `#% reduced Amount
Recovered`, four more) runs the other way, and a single `>=` would make a Member on those groups stop
discriminating at all: pick Tier 1, get a bound of 30, and every worse tier's larger number satisfies it.
Not unsafe — you Latch, nothing is destroyed — but useless, which is worse than the failure the
fail-closed rule was protecting against.

The "never varies" row is not a special case bolted on. It is what makes flag stats (26 mods in scope
with `min = max = 1`, rendering no number) and constant stats correct rather than merely tolerated, and
it is what makes `ChanceToAvoidFreezeAndChill` resolve to one bound against one rendered value.

**Floors are `f64`, not integers.** Mana regeneration and life leech carry fractional display values on
the shipped jewel base.

### 3 · Multi-stat mods: every varying stat, AND

Every stat that carries a Threshold must satisfy its own bound. This reproduces exactly what comparing
tier ordinals meant, because a tier *is* the package of its bands.

Verified rather than assumed: per-stat floor monotonicity holds for **every stat index of all 66 groups**
in `data/ghastly-eye-jewel.json`, so an AND over the picked tier's bounds can never Miss a roll from a
better tier. The census's 17 same-`required_ilvl` pairs are the only counter-candidates anywhere in
scope, and they have *equal* bounds (`Strength9` +(51-55) and `StrengthEssence7_` +(51-58) both start at
51), so they are monotone too. The invariant is asserted at generation time — a tripwire, not a fix.

Constraining one number of a hybrid mod and waiving the rest — armour off a body armour, never mind the
evasion — is a real product idea and is **fog**. It needs picker UI per stat, and the rule model is one
Threshold per Member.

### 4 · The cross-check compares the affix name, and keeps halting

`AnnotationDisagrees` is the instrument the map's trust story leans on: #22 killed the poedb
hand-verification half for armour, and there is no scraper, so the runtime cross-check is doing most of
the verification work for 120 Base Groups. #39 required that it not be weakened as a side effect.

Comparing ordinals cannot keep that job. Our ladder is derived, not observed, so a disagreement cannot
distinguish "the band is wrong" from "the numbering is wrong", and those need opposite responses — and
241 tiers renumber on a question the export cannot answer.

**The affix name can.** It comes from the same RePoE row as the band, the game prints it verbatim, and
`Stalwart` stays `Stalwart` however the ladder is numbered. So:

> `AnnotationDisagrees` fires when the annotated affix name is not the `affix_name` of **any** tier whose
> bands accept the roll. It stays `halt_worthy()`.

That works under a straddle without having to pick a winner, and it verifies precisely what a Threshold
depends on — that some row in our data is the mod the game says this is. It still catches the dangerous
confusion: if the game says `of the Essence` and every accepting row is a normal tier, that is a real
disagreement. It is a *stronger* check than the ordinal, because a name pins a RePoE row while an ordinal
only pins our derivation of one.

`Annotation::affix_name()` has existed in `item.rs` since the parser was written and has never been read.

The ordinal comparison is **kept as a separate, non-halting diagnostic.** Logging it accumulates free
evidence toward the census's largest open question — does GGG's ladder count essence-only mods? — without
letting that question stop the app.

### 5 · The only ambiguity left is arity, and it Halts

`NoTierMatched` and `ManyTiersMatched` stop deciding Verdicts and survive as information, which is what
`verdict.rs`'s own module doc already wanted ("it never reaches a Verdict"). A roll inside two bands is no
longer a question; a roll inside no band is no longer an anomaly, because a value wedged between two
tiers of a flat ladder is the normal shape of an essence roll.

One genuine ambiguity remains: **the count of rendered values may not match the count of Thresholds.**
Decision 2's exclusion rule disposes of the known case, but a residual mismatch means the app cannot map
a number to a bound at all. That is a data fault, and it is **halt-worthy**, with a diagnostic naming the
group and both counts.

`Unknown` was the tempting middle answer and is rejected: the condition is a property of the data rather
than of the Read, so a Resync reproduces it exactly and ADR 0002's three-consecutive-Unknowns rule Halts
on the third — the same destination, three Trigger Presses later, with the log blaming the reads.

Note this is *stricter* than the shipped behaviour and deliberately so. Today an arity mismatch fails
closed to a `Hit`, which for `ChanceToAvoidFreezeAndChill` means Latching on every single Roll with
nothing in the log explaining the loop.

### 6 · The persisted setup stores both, and re-resolves

The map decided the last setup persists and is **re-validated against the pool data before use rather
than trusted**. This is what it validates: the persisted Member carries **the picked Tier and the
Threshold it resolved to**.

On load, re-resolve the Tier against current data. Identical bounds → restore silently, which is every
ordinary restart. Different bounds → restore the Member but mark it **unconfirmed**, so the Rule cannot
arm until the human has looked, with the window naming the old and new numbers.

Storing only the Threshold — the census's own recommendation — is blind to a GGG **rebalance**, which
genuinely should move what "Tier 2 or better" means. Storing only the Tier reintroduces the exact hazard
this ADR removes: regenerate the pools with essence mods in the ladder and the acceptance craft's own
Member silently changes number. Storing both is what lets the app tell a renumber from a rebalance
instead of guessing which happened.

Neither outcome would be unsafe — a stale bound either Latches later than asked or Latches slightly below
the tier named, and both *stop* the craft. This is a correctness-and-annoyance call, which is why it
surfaces rather than silently picking a side. The cost is bounded: pool data changes only when the pool
files change, so the prompt is rare, and the reason persistence exists at all — a ~4-minute
merge → CI → auto-update loop that restarts the app constantly — is untouched.

### 7 · The odds ask the Threshold's question

The map charted odds by seeded Monte Carlo over the whole Rule. The simulator samples a tier by
`spawn_weight`, samples values inside that tier's bands, and evaluates the Rule with **the same
comparison `assess()` uses**. There is no separate tier-threshold path, so the panel and the hit test
cannot describe different crafts.

A simulator has to sample a value inside the tier it drew in order to evaluate the Rule at all, so the
within-band distribution convention is not a cost of this decision — it was already required.

**The 1-in-272 oracle survives exactly.** For `MinionAddedPhysicalDamage` at Tier 1 the Threshold is
(23, 33); every T1 roll clears it and no roll of T2 (18-21 to 24-27) or below can reach it, so the
qualifying weight is **175 under both models**. `crates/core/tests/odds.rs` remains a valid regression
test for the simulator, which is the whole reason the map kept it.

More generally: across all 66 shipped groups and every threshold, with bands normalised, the two models
give **identical odds — zero divergences**. No better tier fails to fully clear its own bound and no
worse tier can reach it, which is [#4](https://github.com/Furizaa/poe-graft/issues/4)'s finding restated
— the jewel base has no overlapping bands. The two models can diverge *only* where bands overlap, which
is exactly the 18 census pairs and the essence cases, and there the ordinal answer was the wrong one.

### 8 · Tier names the ask, numbers name the result

ADR 0003 gives `crates/core` the window's sentence. It becomes:

- **Armed** — the Tier leads, because that is what was picked, and the sentence states the **Threshold**
  rather than the picked tier's full band: *"Looking for increased Movement Speed at Tier 2 or better —
  at least 30%."* One character shorter than 0003's rendering and a different claim.
- **After a Hit** — what landed leads, and a tier is named only when exactly one tier accepts the roll:
  *"Landed 30% — Tier 2."* When none or several accept: *"Landed 32% — you asked for 30%."*

`Assessment::tier()` becomes a set and stops being load-bearing.

This spends words in the one place ADR 0003 is most protective of, and it is the right place to spend
them: the moment the human decides whether to keep the item is the moment both halves matter.

## Consequences

- **`crates/core` loses its tier comparison from the decision path entirely.** `Target::tier_threshold()`
  and `derived <= target.tier_threshold()` at `verdict.rs:296` both go. The tier is still *computed*, for
  the sentence and for the non-halting ordinal diagnostic, but nothing consults it to reach a Verdict.
  [#29](https://github.com/Furizaa/poe-graft/issues/29) owns the change.
- **`CONTEXT.md` retires `Tier Threshold` and adds `Threshold`.** Two words for two things, because
  blurring them is the confusion this ADR exists to end. `Tier`, `Hit` and `Diagnostic` are amended as
  consequences: `Tier` no longer claims to be derived in order to decide anything, `Hit`'s "Ambiguity
  resolves to a Hit" is narrowed to the one ambiguity that remains, and `Diagnostic`'s list now says the
  cross-check compares an affix name.
- **Two bugs were found while deciding this, and neither is this ADR's to fix.**
  `Annotation::parse` makes `(Tier: N)` mandatory (`item.rs:195` ends in `?`) despite the doc comment two
  lines above calling it optional — so an essence-only mod's annotation line, which #22 established
  carries no tier, falls through to `unrecognised` → `UnrecognisedLine` → `halt_worthy()`. With Advanced
  Mod Descriptions now load-bearing for identification, **acceptance run 2 halts on the annotation line**
  before tiers are even in question. Separately, `ChanceToAvoidFreezeAndChill` can never match a tier in
  the shipped app, for the arity reason in decision 2. Both are filed as children of #21; the first
  blocks #29.
- **One census recommendation is knowingly *not* taken.** #23's item 4 asked for one in-game capture to
  settle whether GGG's ladder counts essence-only mods, before the essence craft is built. This ADR
  demotes that from a blocker to a curiosity: 241 tier numbers no longer gate anything, because no
  Verdict, no Threshold and no odds figure depends on the ordinal. The capture is still worth taking —
  the non-halting ordinal diagnostic in decision 4 will do it for free, over a whole craft's worth of
  Rolls, instead of one hand-copied item.
- **This ADR is slightly more fail-closed than tier logic, in the direction `CONTEXT.md` mandates.** A
  roll inside a worse tier's band that clears the better tier's bound reads as a `Hit`. That is the safe
  direction, and it is arguably the correct one: the human asked for numbers.
- **The three research branches landed on `main`** so this ADR's central citation resolves without a
  checkout. That question was left open on the map; it is now answered the way map #1 answered it.
