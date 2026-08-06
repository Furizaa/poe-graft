# The affix-count distribution for a Rare armour reforge, and what an Essence does to it

Research for [#24](https://github.com/Furizaa/poe-graft/issues/24). Captured 2026-08-06.

Primary sources are the **source code of open-source crafting simulators**, read at a pinned commit
— not write-ups about them. Where GGG data settles a question, RePoE **3.29.2.1** is cited instead.

Artefacts: none. This document is the whole deliverable; the numbers below are what
[`scripts/build-mod-pool.py`](../../scripts/build-mod-pool.py) should emit and what the Monte Carlo
simulator in `crates/core` should implement.

---

## Verdict

**A Rare armour reforge produces 4, 5 or 6 affixes with relative weights 8 : 3 : 1** — 66.67% /
25% / 8.33%, mean 4.4167. **It does not vary by armour slot, or by item class at all** except that
jewels and flasks are different buckets. Three simulators agree character for character, though
their histories show they are **not demonstrably independent** of each other; the strongest evidence
is that kalandralang threw away its own hand-measurement to adopt this.

**The count is drawn first, and then filled by repeated single draws from one combined
prefix+suffix pool** — the prefix/suffix split is *emergent*, never drawn. All three simulators
implement the same inner loop, and all three rebuild the eligible pool before every draw, removing
Mod Groups already on the item and removing whichever side has hit its cap. See
[The algorithm](#the-algorithm).

**An Essence places its guaranteed mod first, and that mod occupies one of the 4–6 slots.** It is
not a bonus. `remaining = drawn_count − mods_already_on_item`, so an Essence craft makes **3, 4 or
5** random draws with the same 8 : 3 : 1 weights. All three simulators agree, and they agree by
each writing that subtraction out explicitly rather than by coincidence.

**One correction to the repo's existing note.** `data/ghastly-eye-jewel.json` says its
`rare_jewel` figure of 3 @ 65 / 4 @ 35 comes from "the two independent open-source crafting
simulators that agree on it (kalandralang `src/item.ml`, PoeCraftLib `StatFactory.cs`)". That is
true of the **`magic` 1:1 split** but **not of `rare_jewel`**: PoeCraftLib has no jewel branch at
all, and neither does any other simulator read here. **`rare_jewel` 65/35 rests on kalandralang
alone.** See [Epistemic status of every number](#epistemic-status-of-every-number). It does not
touch the 1-in-272 oracle, which is a Magic craft.

**The 1-in-272 oracle survives, provably.** `pool.rs`'s closed form is not a convention that the
Monte Carlo simulator has to be coaxed into matching — it is the *exact* marginal of the algorithm
above, applied to a Magic jewel. The identity is derived in
[Why the oracle still holds](#why-the-oracle-still-holds). A correct general simulator must
converge on 1 in 272.

---

## Sources read

| Simulator | Language | Commit read | Dated |
| --- | --- | --- | --- |
| [doomeer/kalandralang](https://github.com/doomeer/kalandralang) | OCaml | `5a6d1fbead0fa20b239febcc69b75bc79e622466` | 2026-01-17 |
| [DanielWieder/PoeCraftLib](https://github.com/DanielWieder/PoeCraftLib) | C# | `07a65914b05665eec972a6f32b322233b85412a4` | 2020-03-19 |
| [srhinos/poe-cli](https://github.com/srhinos/poe-cli) | Python | `524a5384e9ae8db94d94147462232972386bc96a` | 2026-03-26 |
| [rickbutton/hinekora](https://github.com/rickbutton/hinekora) | TypeScript | `76120053218e56120894dd070a233f734c065647` | 2026-07-21 |

Read as full clones, so the `git log -S` archaeology below is real rather than a shallow-boundary
artefact. Line references are to the commits above.

kalandralang and PoeCraftLib are the two the repo already cites. **poe-cli is new here** and is the
third vote on the armour weights. **hinekora is a static checker, not a sampler** — it carries no
count weights, but it models affix *caps* more carefully than any of the other three and is cited
only for those.

Searched and rejected: `Deltaidiots/poe_rl` (Path of Exile **2** ring crafting — different game,
out of scope per #21), `khrave/PoECraftingSim`, `Kamil-School/poe-crafting-sim`,
`3uclid3/path-of-crafting`, `joonazan/crafting-path` — none contains a generation model. A GitHub
code search for `affix_count_odds` returns only this repo; for `RareAffixCountOdds`, only
PoeCraftLib.

---

## The weights

### Rare, non-jewel, non-flask — this is armour

kalandralang, `src/item.ml:684-689`, inside `spawn_additional_random_mods`:

```ocaml
          else
            let w4 = 8 in
            let w5 = 3 in
            let w6 = 1 in
            let i = Random.int (w4 + w5 + w6) in
            if i < w4 then 4 else if i < w4 + w5 then 5 else 6
```

PoeCraftLib, `src/Currency/StatFactory.cs:27-32`:

```csharp
        public static readonly Dictionary<int, int> RareAffixCountOdds = new Dictionary<int, int>()
        {
            {4, 8},
            {5, 3},
            {6, 1}
        };
```

poe-cli, `poe/services/repoe/sim.py:101-111`:

```python
    _RARE_MOD_COUNTS: typing.ClassVar[list[int]] = [4, 5, 6]
    _RARE_MOD_WEIGHTS: typing.ClassVar[list[int]] = [8, 3, 1]

    def _rare_mod_count(self) -> int:
        """Sample a rare item mod count using GGG's 58/28/14 distribution."""
        return random.choices(self._RARE_MOD_COUNTS, weights=self._RARE_MOD_WEIGHTS, k=1)[0]
```

Three sources, one distribution: **4 @ 8, 5 @ 3, 6 @ 1**.

Note that poe-cli's *docstring* says "58/28/14" while its *code* says 8/3/1 = 66.7/25/8.3. The
docstring is wrong about the code it sits on and should be given no weight at all — it is a
comment, not a source.

### Where the constant came from, as far as the histories show

Three codebases agreeing is worth less if they copied each other, so I traced each one (full
clones, not the shallow ones — `git log -S` on a shallow clone reports the boundary commit and
means nothing).

- **PoeCraftLib carries 8/3/1 in its Initial Commit, `d97b74a`, 2019-12-10**, inline in
  `ChaosOrb.cs` as `int fourMod = 8; int fiveMod = 3; int sixMod = 1;`. It moved to the named
  `RareAffixCountOdds` dictionary in `efe86df`, 2020-01-12. No citation, then or since.
- **kalandralang adopted it on 2022-04-16 in `abc75e6`**, and this is the interesting one, because
  of what it *replaced*. The previous code was a hand-measurement carrying its own confession:

  ```ocaml
  (* Tested a small sample, counting how many items had 4, 5, 6 mods after a chaos. *)
  (* TODO: better sample *)
  let w4 = 56 in
  let w5 = 22 in
  let w6 = 3 in
  ```

  The commit message for the replacement is `- non-jewels have 8/12 to roll 4 mods, 3/12 to roll 5,
  1/12 to roll 6`, and the same commit introduced the jewel branch as `65%` / `35%`. So
  kalandralang's author had 81 observations of his own — normalising to 69.1 / 27.2 / 3.7, which
  brackets 66.7 / 25 / 8.3 loosely and undershoots the 6-mod case badly on that sample size — and
  **discarded them in favour of exact twelfths**. Someone stating a distribution in twelfths is
  quoting a table, not a measurement.
- **poe-cli, 2026**, is the latest and is the one most likely to be downstream of the other two.

**So the honest reading is: three sources agree, and they are not demonstrably independent.** The
chronology runs PoeCraftLib 2019 → kalandralang 2022 → poe-cli 2026, and any of the later two could
have copied an earlier one or a shared community source. What raises this above "one number copied
three times" is that kalandralang traded away its own data for it.

One implementation warning from the same history: `abc75e6` shipped the cumulative comparison as
`if i < w4 then 4 else if i < w5 then 5 else 6` — `w5`, not `w4 + w5` — which makes 5-mod items
*impossible* and 6-mod items 4/12. It took two days and a separate commit (`af304f6`, 2022-04-18,
"Fix weighting of 5-mod items") to catch. Worth a test in `crates/core` that asserts the sampler's
three frequencies, not just its support.

### The other buckets, for completeness

kalandralang is the only simulator that branches, `src/item.ml:664-689`:

| Bucket | Counts and weights | kalandralang branch |
| --- | --- | --- |
| Magic, anything | 1 @ 50, 2 @ 50 | `Magic ->` |
| Rare **flask** (`domain = Flask`) | 1 @ 50, 2 @ 50 | `if item.base.domain = Flask` |
| Rare **jewel** (`Jewel` / `AbyssJewel`) | 3 @ 65, 4 @ 35 | `else if Base_item.is_jewel item.base` |
| Rare **everything else** | 4 @ 8, 5 @ 3, 6 @ 1 | `else` |

The Rare-flask row is unreachable in Path of Exile 1 — a non-unique flask stops at Magic — so it is
defensive code, and irrelevant to #21's flask craft, which alt-spams a Magic flask and therefore
uses the Magic row.

The Magic 1:1 split is confirmed by all three: kalandralang `w1 = 50 / w2 = 50`, PoeCraftLib
`MagicAffixCountOdds = {1:1, 2:1}` (`StatFactory.cs:34-38`), poe-cli `random.randint(1, 2)`
(`sim.py:333`). This is the number the shipped jewel file already carries and the number the
1-in-272 figure rests on; it is now three-source rather than two.

### Does it vary by item class or slot?

**No — not across the five armour slots, and not across armour, weapons or jewellery.** The only
branches in any simulator are jewel and flask. Boots, Gloves, Helmets, Body Armours and Shields all
take the `else` branch. Nothing in any of the four codebases keys the count off slot, base, tag set
or item level.

**The caps, however, do vary — and #21's "Not yet specified" jewellery work will trip on it.**
Three sources give 3 prefixes + 3 suffixes for a Rare non-jewel: kalandralang
`max_prefix_count` (`item.ml:84-94`), PoeCraftLib `IsFull` / `GetAffix` (`StatFactory.cs:181-190`,
`AffixManager.cs:60-62`), poe-cli `DEFAULT_MAX_PREFIXES = 3` with `{"Jewel": 2, "AbyssJewel": 2,
"Flask": 1}` overrides (`poe/services/repoe/constants.py:35-46`). But hinekora goes further
(`packages/core/src/check/counts.ts:42-77`, `packages/data/scripts/ingest-poe1.mjs:75-90`): a
base's implicits can carry `local_maximum_prefixes_allowed_+` /
`local_maximum_suffixes_allowed_+`, which shift the per-side caps while the *total* cap stays at 6.

I checked that against RePoE 3.29.2.1 directly. **Exactly 8 released bases carry those stats, and
not one of them is armour:**

| Base | Δprefix | Δsuffix |
| --- | --- | --- |
| Cogwork Ring | −1 | +1 |
| Geodesic Ring | +1 | −1 |
| Composite Ring | +3 | −3 |
| Manifold Ring | +1 | −2 |
| Ratcheting Ring | −3 | +3 |
| Helical Ring | −2 | +1 |
| Simplex Amulet | −2 | −1 |
| Focused Amulet | −1 | −2 |

So for #21's Base Groups — 81 Boots, 79 Gloves, 94 Helmets, 122 Body Armours, 97 Shields, all
released `domain: item` bases — **3 + 3 is universal and 4–6 is always reachable.** When the
project reaches rings and amulets it stops being universal: a Simplex Amulet caps at 1 + 2 = 3
total, so it cannot hold the 4 the count roll will ask for. kalandralang, PoeCraftLib and poe-cli
all model this incorrectly (flat 3/3); only hinekora gets it right, and hinekora does not sample.
Recommendation for that future ticket: clamp the drawn count to `min(6, prefix_cap + suffix_cap)`,
and rely on the fail-soft in [The algorithm](#the-algorithm) meanwhile.

---

## The algorithm

This is the part that changes the simulator's inner loop rather than a constant, so it is stated as
pseudocode. All four claims below are verified in all three sampling simulators.

1. **The total count is drawn first.** One weighted draw over `{4:8, 5:3, 6:1}`.
2. **Prefixes and suffixes are not drawn independently, and the total is not split.** The count is
   filled by `n` single draws from **one combined pool** holding both prefixes and suffixes,
   weighted by spawn weight. The split falls out of the relative pool weights and the caps.
3. **The pool is rebuilt before every draw**, excluding (a) every Mod Group already present on the
   item, (b) every mod on a side that has reached its cap.
4. **A draw that finds nothing empty fails soft** — the loop stops and the item ends with fewer
   affixes than the count asked for, rather than erroring or retrying.

```
reforge_rare(item, pool, ilvl):
    item.mods.clear()
    n ← weighted_choice({4: 8, 5: 3, 6: 1})
    while count(item.mods) < n:
        eligible ← [ m for m in pool
                     if m.required_ilvl ≤ ilvl
                     and m.spawn_weight_for(item.tags) > 0
                     and m.group ∉ groups(item.mods)               # one mod per Mod Group
                     and not (m.is_prefix and prefixes(item) ≥ max_prefixes)
                     and not (m.is_suffix and suffixes(item) ≥ max_suffixes) ]
        if total_weight(eligible) == 0: break                      # fail soft, not an error
        m ← weighted_choice(eligible, by = m.spawn_weight_for(item.tags))
        item.mods.add(roll_values(m))
```

Where each simulator says this:

- **kalandralang.** `spawn_additional_random_mods` (`item.ml:658-696`) draws `final_mod_count`,
  then `spawn_n_mods (final_mod_count - prefix_and_suffix_count item)`, each step calling
  `spawn_random_mod ~fail_if_impossible: false`. `spawn_random_mod` (`item.ml:504-508`) calls
  `mod_pool` — which is rebuilt from scratch every call — and hands the result to
  `random_from_pool` (`misc.ml:18-32`), a plain cumulative-weight walk over the *filtered* list.
  The two filters that matter are in `mod_pool`'s `can_spawn_mod` (`item.ml:397-413`):

  ```ocaml
    let already_has_mod_group = has_mod_group_prefix_or_suffix modifier.Mod.groups item in
    ...
    else if Mod.is_prefix modifier && (not allow_prefix || prefix_count >= max_prefix) then
      None
    else if Mod.is_suffix modifier && (not allow_suffix || suffix_count >= max_suffix) then
      None
    ...
    else if already_has_mod_group then
      None
  ```

- **poe-cli.** `_roll_item` (`sim.py:262-280`) is the loop verbatim; `_build_mod_pool`
  (`sim.py:135-186`) does the filtering, including `if mod["group"] in existing_groups: continue`
  and `if affix == "prefix" and item.open_prefixes <= 0: continue`; `_weighted_pick`
  (`sim.py:188-201`) sums and walks.

- **PoeCraftLib.** `AddExplicits` (`StatFactory.cs:117-141`) draws the count then
  `while (item.Stats.Count < affixCount) { if (!AddExplicit(...)) break; }`. It reaches the same
  distribution by *subtraction* rather than filtering: `AffixManager.GetAffix`
  (`AffixManager.cs:48-129`) precomputes and caches per-group weight sums, then computes
  `prefixSkipAmount` / `suffixSkipAmount` — the weight of every already-present group, or the whole
  side's weight if that side is at cap — and draws
  `random.NextDouble() * (TotalWeight − prefixSkip − suffixSkip)`, skipping present groups while
  walking. Mathematically identical to filter-then-renormalise; more code, same answer. It caches
  the pool but not the exclusions, which is the right split and worth copying.

**Consequence for `crates/core`: draw without replacement at Mod Group level, renormalising over
the remaining eligible weight after every draw.** All three simulators do exactly that. There is no
"draw 4 independently and discard collisions" shortcut in any of them, and there should not be one
here.

### The prefix/suffix split is never forced

None of the three draws a split, and none of them enforces "at least one prefix and one suffix" for
a Rare. poe-cli has a `require_both_affixes` fixup (`sim.py:281-301`) that Chaos, Alchemy and
Harvest reforge pass — but it is a **no-op for any Rare**: with caps of 3 + 3 and a count of 4 or
more, the cap filter already guarantees both sides are non-empty. It is also **not** passed by
`essence_roll`. So for #24's question it changes nothing, and the repo should not implement it.

The split is therefore a *result* the simulator reports, not an input. Worth stating because #21's
"provably impossible rules are refused at the first Read" check reasons over `affix_slots`, and the
right reading of a rule needing 4 suffixes on boots is "impossible" (cap 3) — not "unlikely because
the split usually gives 2".

---

## What an Essence does

**The guaranteed mod is placed first and consumes one of the 4–6 slots.** It is a replacement, not
an addition. Stated as an algorithm:

```
essence_reforge(item, essence, pool, ilvl):
    item.rarity ← Rare
    item.mods.clear()                                   # every explicit is destroyed
    g ← essence.mod_for(item.item_class)                # a single fixed mod id, no tier roll
    if g is None: refuse                                # this essence does not apply to this class
    item.mods.add(roll_values(g))                       # occupies one prefix or suffix slot
    n ← weighted_choice({4: 8, 5: 3, 6: 1})             # same weights, unchanged by the Essence
    while count(item.mods) < n:                         # ⇒ n − 1 further draws: 3, 4 or 5
        ... exactly the loop from `reforge_rare`, so g's Mod Group is already excluded
```

Three sources, each writing the subtraction out:

- **kalandralang.** `interpreter.ml:526-541` resolves the essence to one mod id and calls
  `Item.reforge_rare ~respect_cannot_be_changed: false ~modifier (Item.set_rarity Rare item)`.
  `reforge_rare` (`item.ml:711-729`) removes all mods, `add_mod modifier item`, then
  `spawn_additional_random_mods`, whose last line is
  `spawn_n_mods (final_mod_count - prefix_and_suffix_count item) item` — the guaranteed mod is
  already counted in `prefix_and_suffix_count`.
- **PoeCraftLib.** `EssenceToCurrency` (`CurrencyFactory.cs:179-205`) builds the step list
  literally: `SetRarity(Rare)`, `RemoveExplicits(All)`, `AddExplicitByItemClass(essence.mods)`,
  `AddExplicits(RareDistribution)` — and `AddExplicits` tops up to the drawn total with
  `while (item.Stats.Count < affixCount)`.
- **poe-cli.** `essence_roll` (`sim.py:490-527`): `_add_mod(item, guaranteed_mod)`, then
  `total_target = self._rare_mod_count()`, `remaining = total_target - len(item.all_mods)`.

So for the odds of the *remaining* Members of a Rule, an Essence craft draws:

| Random draws after the guaranteed mod | Probability |
| --- | --- |
| 3 | 8/12 = 66.67% |
| 4 | 3/12 = 25.00% |
| 5 | 1/12 = 8.33% |

Three further facts the simulator needs, all from GGG data rather than simulator source:

- **The guaranteed mod is a fixed mod id at a fixed tier with a fixed generation type — it is not
  a random draw and often not even a random *value*.** RePoE `essences.json` maps essence → item
  class → one mod id. Deafening Essence of Zeal on Boots gives `MovementVelocityEssence7`, whose
  stat range in `mods.json` is `min 32, max 32`. Shrieking gives `MovementVelocity5`, range
  `30, 30`. For #21's Rule model this means the guaranteed **Member is deterministic**: with the
  right essence it is a Hit on every single Roll, at a tier the essence fixes. The Monte Carlo
  simulator should treat it as certainty, not as a draw — which is also exactly what #21's "the
  essence's guaranteed mod is an ordinary Member with no special treatment" decision needs in order
  to produce a sane number.
- **`MovementVelocityEssence7` cannot spawn randomly at all** — its only `spawn_weights` entry is
  `{"tag": "default", "weight": 0}`, and `is_essence_only: true`. So the top-tier essence mods must
  be in the pool file for the matcher (per #21: an unrecognised line Halts) while being **excluded
  from the random draw** by the `spawn_weight > 0` filter the algorithm above already applies. The
  natural home for them is a sibling of the existing `non_alteration_pools` block.
- **Movement Speed on Boots is a *prefix***, not a suffix (`MovementVelocity5.generation_type =
  prefix`). #21's acceptance rule is therefore 1 prefix + at least 2 suffixes, comfortably inside
  3 + 3 — but the slot-cap feasibility check must read the generation type from the data, not from
  intuition.
- **Only Shrieking and Deafening essences can be applied to an already-Rare item**, which is the
  narrowed form of the assertion #21 recorded as owner-asserted-not-verified. PoeCraftLib
  `CurrencyFactory.cs:185-187`:

  ```csharp
            var rarityRequirement = essence.Level >= 6 ?
                new List<RarityOptions>() {RarityOptions.Normal, RarityOptions.Rare} :
                new List<RarityOptions>() { RarityOptions.Normal };
  ```

  `essence.Level` is RePoE `essences.json`'s `level` field (`EssenceFactory.cs:50`). In 3.29.2.1
  that field runs 1–8: Whispering 1 … Screaming 5, **Shrieking 6, Deafening 7**, corrupted 8 —
  cross-checked by name (`Screaming Essence of Zeal` level 5, `Shrieking` 6, `Deafening` 7) and
  matching kalandralang's own `level` enum (`essence.ml:11-28`). kalandralang does **not** model
  this restriction — its `Essence` case just calls `Item.set_rarity Rare` unconditionally — so this
  is PoeCraftLib alone, corroborated by a GGG data field. It is a picker constraint, not an odds
  input: the app should only offer Shrieking and Deafening for a Rare craft.

---

## No-duplicate-Mod-Group

**Yes, all three sampling simulators model it, and all three model it as this project assumes: one
mod per group per item, enforced at draw time by removing the group from the pool.** Quoted above
in [The algorithm](#the-algorithm) — kalandralang `already_has_mod_group`, poe-cli
`if mod["group"] in existing_groups: continue`, PoeCraftLib `if (existingGroups.Contains(group.Key))
continue` plus the matching `prefixSkipAmount` subtraction.

Two details worth carrying into the implementation:

- **A mod can belong to more than one group.** kalandralang models `Mod.groups` as a *set* and
  tests with `not (Id.Set.disjoint groups modifier.Mod.groups)` (`item.ml:136-144`), and its commit
  `6ff8d0b compatibility with new data: multiple groups` is exactly the migration from a single
  group to a set. PoeCraftLib and poe-cli both use a single `Group` string and would get this wrong.
  RePoE's `mods.json` carries `groups` as an array, so the pool builder should key exclusion on the
  whole array, not on `groups[0]`.
- **Implicits do not participate.** kalandralang's `has_mod_group_prefix_or_suffix` matches only
  `Prefix | Suffix` generation types, so an implicit sharing a group with an explicit does not block
  it. Consistent with #21's decision to consult only the explicit-mod section.

Everything else in the eligible-pool filter is already in the shipped data model: `required_ilvl`
(`A tier can spawn iff item level >= required_ilvl`, already a note in `matching.notes`) and the
first-matching-tag `spawn_weights` rule that `build-mod-pool.py` self-check #1 already reproduces.

---

## Why the oracle still holds

#21 makes `crates/core/tests/odds.rs`'s 1 in 272 the simulator's oracle: "a single-Member rule
against the jewel pool must reproduce it within tolerance". That only means something if the
existing closed form is what the general algorithm actually converges to. It is — exactly, not
approximately.

Take a Magic jewel, caps 1 + 1, prefix pool total `P`, suffix total `S`, and a target **prefix**
group whose qualifying tiers sum to weight `w`. Run the algorithm above:

- **1-affix roll** (probability ½): one draw from the combined pool. `P(hit) = w / (P + S)`.
- **2-affix roll** (probability ½): draw 1 from the combined pool, then draw 2 from what is left.
  - draw 1 lands in the target group: `w / (P+S)` — hit.
  - draw 1 lands in another prefix: `(P − w) / (P+S)` — the prefix cap is now full, draw 2 is
    restricted to suffixes, miss.
  - draw 1 lands in a suffix: `S / (P+S)` — the suffix cap is now full, draw 2 is restricted to the
    prefix side whose eligible weight is still the full `P` (prefix and suffix groups are disjoint
    on this Base, verified in [`mod-tier-data.md`](mod-tier-data.md)), so `w / P`.

  ```
  P(hit | 2 affixes) = w/(P+S) + (S/(P+S))·(w/P)
                     = w·(P + S) / (P·(P+S))
                     = w / P
  ```

That is `Odds::conditional` verbatim, and

```
p_click = ½·(w/P) + ½·(w/(P+S))
```

is `pool.rs:438-439` verbatim — `p_two * conditional + p_one * (weight / both)`. Checked
numerically at ilvl 83 with `P = 38950`, `S = 22100`, `w = 175`: the two-affix branch gives
`175/61050 + (22100/61050)·(175/38950) = 0.00449294`, and `w/P = 0.00449294`.

So the closed form is the exact marginal of the sequential algorithm for a Magic jewel, and a
correct general Monte Carlo simulator will reproduce 1 in 272 to sampling error. The oracle is a
real regression test, not a calibration.

**The same identity does not generalise to Rare**, which is why #21 chose Monte Carlo and was right
to. It collapses because it relies on the second draw seeing the *undiminished* `P` — true only
when at most one prefix can be present. On a Rare armour with 3 + 3, the third and fourth draws see
a prefix pool reduced by two or three consumed groups, the reduction depends on *which* groups were
consumed, and the count itself is random. There is no clean product form; sampling is the honest
answer.

A cheap unit-test constant for the count sampler alone, needing no pool at all: the mean drawn
count is `(4·8 + 5·3 + 6·1)/12 = 53/12 = 4.416̄`, and `P(6) = 1/12`. For the jewel bucket the mean
is `(3·65 + 4·35)/100 = 3.35`.

---

## The form `build-mod-pool.py` should emit

The generator currently hardcodes the jewel's block (`scripts/build-mod-pool.py:269-276`). Make it
a bucket table plus a lookup, so a Base Group's item class picks the row:

```python
# Relative weights for how many affixes a freshly reforged item gets, by rarity.
# Which Rare row applies is a function of item class only — never of slot, base or ilvl.
# Provenance and epistemic status: docs/research/rare-affix-count-odds.md
MAGIC_AFFIX_COUNT_ODDS = {'1': 1, '2': 1}
RARE_AFFIX_COUNT_ODDS = {
    'Jewel':      {'3': 65, '4': 35},          # 2 + 2 slots
    'AbyssJewel': {'3': 65, '4': 35},
    'Flask':      {'1': 1, '2': 1},            # 1 + 1 slots; unreachable in PoE 1
}
DEFAULT_RARE_AFFIX_COUNT_ODDS = {'4': 8, '5': 3, '6': 1}   # all five armour slots land here

NOTE = ('Relative weights for how many affixes a freshly reforged item gets. Not present in GGG '
        'data files. magic 1:1 and rare 8:3:1 agree across three open-source simulators '
        '(kalandralang src/item.ml, PoeCraftLib src/Currency/StatFactory.cs, poe-cli '
        'poe/services/repoe/sim.py) which are not demonstrably independent of each other; the '
        'jewel 3:65/4:35 rests on kalandralang alone. The count is drawn first and then filled by '
        "single draws from one combined prefix+suffix pool; an Essence's guaranteed mod occupies "
        'one of those slots rather than adding to them. '
        'See docs/research/rare-affix-count-odds.md.')


def affix_count_odds(item_class):
    return {
        'magic': MAGIC_AFFIX_COUNT_ODDS,
        'rare': RARE_AFFIX_COUNT_ODDS.get(item_class, DEFAULT_RARE_AFFIX_COUNT_ODDS),
        '_note': NOTE,
    }
```

For a Boots / Gloves / Helmet / Body Armour / Shield Base Group that emits:

```json
  "affix_count_odds": {
    "magic": { "1": 1, "2": 1 },
    "rare": { "4": 8, "5": 3, "6": 1 },
    "_note": "…"
  },
```

**One schema decision this forces, and the recommendation.** The shipped jewel file keys its Rare
row `rare_jewel` — the *bucket* name leaked into the *key*, so a reader cannot find the Rare row
without first knowing which bucket the Base is in. Since the file is per Base Group the bucket is
never ambiguous, so the key should just be `rare`. Recommend **renaming `rare_jewel` → `rare` when
the pipeline is regenerated**. It is free today: `pool.rs`'s `RawAffixCountOdds`
(`crates/core/src/pool.rs:470-475`) deserialises only `magic`, and serde ignores unknown keys, so
nothing reads `rare_jewel` and nothing breaks either way. If the rename is unwanted, the fallback
is for `pool.rs` to look up `rare` then `rare_jewel` — but that is a reader carrying a
naming accident forever.

`affix_slots` needs the same treatment and has no simulator ambiguity: `{"magic": {1, 1}, "rare":
{3, 3}}` for all five armour slots, `{2, 2}` for jewels. **It is not in RePoE** — `base_items.json`
has no affix-slot field of any kind (checked; the closest thing is the per-base implicit deltas
tabulated above) — so the caps are as community-derived as the counts, just unanimously so.

---

## Epistemic status of every number

The point of this section is that no number below should be quoted without its column.

| Number | Status |
| --- | --- |
| Rare non-jewel: 4 @ 8, 5 @ 3, 6 @ 1 | **Three agreeing simulator sources**, read at pinned commits, but **not demonstrably independent** — PoeCraftLib 2019-12 predates the other two. No GGG data file contains it. Strengthened by kalandralang discarding its own 81-item sample in favour of it. |
| Magic: 1 @ 1, 2 @ 1 | **Three agreeing simulator sources**, same independence caveat. Was recorded as two; poe-cli is the third. |
| Rare jewel: 3 @ 65, 4 @ 35 | **One source only — kalandralang**, `abc75e6`, 2022-04-16, uncited. The repo's existing note naming PoeCraftLib as a second is wrong; PoeCraftLib has no jewel branch. |
| Rare flask: 1 @ 1, 2 @ 1 | **One source only — kalandralang.** Unreachable in PoE 1; do not ship it as if it mattered. |
| Count drawn first, then filled from one combined pool | **Three agreeing simulator sources**, by three different implementation strategies (filter, filter, weight-subtraction). Strong. |
| Essence: guaranteed mod occupies a slot | **Three agreeing simulator sources**, each writing `n − already_present` explicitly. Strong. |
| Essence weights unchanged by the Essence | **Three agreeing simulator sources** — all three call the same unmodified count draw. |
| One mod per Mod Group, enforced at draw time | **Three agreeing simulator sources.** Also stated as fact in `mod-tier-data.md` from RePoE's `groups` array. |
| A mod may be in several groups; exclude on the whole set | **One source — kalandralang**, plus RePoE's array-typed `groups` field. The other two would get it wrong. |
| Rare caps 3 + 3, jewel 2 + 2, flask 1 + 1 | **Three agreeing simulator sources**, four counting hinekora. **Not in RePoE.** |
| 8 ring/amulet bases shift the per-side caps | **GGG data** — RePoE 3.29.2.1 `mods.json` / `base_items.json`, `local_maximum_{prefixes,suffixes}_allowed_+`. Corroborated by hinekora. Only 1 of 4 simulators models it. |
| No armour base shifts caps | **GGG data**, RePoE 3.29.2.1, scanned exhaustively over released bases. |
| Only Shrieking (level 6) and Deafening (7) apply to a Rare item | **One simulator source — PoeCraftLib** — reading a **GGG data** field (`essences.json.level`). kalandralang does not model the restriction; it does not contradict it. |
| Essence guaranteed mod is one fixed mod id, fixed tier, sometimes fixed value | **GGG data**, RePoE 3.29.2.1 `essences.json` + `mods.json`. |
| Movement Speed on Boots is a prefix | **GGG data**, RePoE 3.29.2.1 `mods.json`. |
| Closed form = exact marginal of the algorithm for Magic | **Derived here** and checked numerically. Not a citation; check the two lines of algebra. |

What would settle the two weakest rows — `rare_jewel` and `rare_flask` — is the same thing
`mod-tier-data.md` already proposed for the Magic split: log affix counts across a real crafting
session. A Rare armour reforge distribution of 8 : 3 : 1 is coarse enough that a few hundred
essence Rolls would distinguish it from any plausible alternative, and the app is going to be
producing exactly those Rolls. That is a better resolution than more reading.

---

## Open questions

- **Where 8 : 3 : 1 *originally* came from.** Traced as far as the histories go — see
  [Where the constant came from](#where-the-constant-came-from-as-far-as-the-histories-show) — which
  ends at PoeCraftLib's initial commit in 2019 with no citation. The trail leaves the codebases
  there. The "twelfths" phrasing suggests a datamined or GGG-stated table upstream of all three, but
  I did not find it, and a forum or wiki restatement would not count.
- **What the game does when the drawn count exceeds the caps.** Only reachable on the 8
  cap-shifted rings and amulets. All three samplers fail soft and produce a short item, which is
  almost certainly not what the game does. Out of scope for this map; a landmine for the jewellery
  work #21 lists as fog.
- **Whether a Rare reforge can ever produce fewer than 4 affixes in-game.** No simulator models it,
  and the fail-soft path is only reachable on pools too small to fill four groups — never on armour.
  Treated here as: the distribution's support is exactly `{4, 5, 6}`.
- **Fossils and Harvest reforges** reach the same count draw in all three simulators
  (kalandralang `reforge_rare ~fossils`, PoeCraftLib `FossilsToCurrency`, poe-cli `fossil_roll` /
  `harvest_reforge`), so the weights are currency-independent as far as the simulators are
  concerned. Only Alterations and Essences are in scope for #21, so this is recorded rather than
  relied on.
