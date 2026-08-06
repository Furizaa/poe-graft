# Do tier ranges overlap within a Mod Group? A census of every in-scope flask and armour Base Group

Research for [#23](https://github.com/Furizaa/poe-graft/issues/23), on the map
[#21](https://github.com/Furizaa/poe-graft/issues/21). Computed 2026-08-06 against the RePoE
`repoe-fork` export, `version.txt` = **3.29.2.1**.

This is a data census rather than a literature review: the primary source is the export itself, and
the primary evidence is a program that reads it. That program is committed at
[`scripts/census-tier-overlaps.py`](../../scripts/census-tier-overlaps.py) — one-off, not part of the
build — and reuses [`scripts/build-mod-pool.py`](../../scripts/build-mod-pool.py)'s own derivation so
the numbers below are the numbers the generator would compute. See
[Reproducing this](#reproducing-this).

---

## Verdict

**The charted per-Member straddle rule survives, for the flask craft cleanly and for the essence craft
only just.** Overlaps are not everywhere in the alteration pools, and they are not deep. Across all
120 in-scope Base Groups there are **18 distinct pairs of tiers whose display bands overlap** once the
three Royale mods are excluded — 8 distinct Mod Groups, 18 (class, group) pairs — and every one is a
one-endpoint or few-value touch: the highest per-Roll ambiguity is **6.5 %**, for two Utility Flask
suffixes. Nothing in the alteration pools would make the app Latch on nearly every Roll.

Essence mods are a different story, and the honest summary is that **for some essence crafts the
straddle is the case on every single Roll**: the elemental avoidance suffixes
(`ChanceToAvoid{Fire,Cold,Lightning}DamageEssence2…7`) step in 1-point increments across 2-wide bands,
so every roll of a middle rank matches two tiers with certainty. Those crafts land on the
annotation branch every time — which is precisely where finding (3) below bites.

**Not one overlapping pair in the alteration pools is numerically indistinguishable.** Zero pairs
have identical display bands, and zero pairs share a `required_ilvl` — so for the flask craft the
game's `(Tier: N)` annotation resolves every straddle the census can find.

**Three findings matter more than the overlaps, and two of them are new risks.**

1. **The blocker is not overlap, it is `match_string` collisions: 53 distinct, on 120 of 120
   in-scope Base Groups.** `pool.rs::from_json` already refuses these with
   `DataError::AmbiguousMatchString`, so as things stand *no* flask or armour pool file would load.
   The cause is structural to armour, not a data glitch: hybrid defence mods render two lines each,
   and every one of those lines is also some other group's only line. This needs a matcher change,
   not a data fix.
2. **Essences do not collide the way the map feared, but they break tier *numbering*.** The charted
   worry — essence movement speed on boots landing on a normal tier — is **false**: Deafening Essence
   of Zeal grants a flat **32 %**, wedged cleanly between the normal 30 % and 35 %. But folding
   essence mods into a group's ladder renumbers **241 normal tiers across 32 (item class, Mod Group)
   pairs**, including `MovementVelocity` on boots, where *30 % increased Movement Speed* is Tier 2
   without essence mods and Tier 4 with them. The annotation cross-check is `AnnotationDisagrees` →
   `Halt`. If our ladder is not the game's ladder, the app halts on the first Roll rather than
   mis-Latching — safe, and unusable.
3. **The annotation is a tiebreak that presupposes agreement.** It can only break a straddle if the
   two candidate tiers carry different numbers *and* our numbering matches GGG's. The census finds 17
   distinct pairs where two tiers share a `required_ilvl` — all of them `Strength` / `Dexterity` /
   `Intelligence` at ilvl 82 on all five armour classes, plus `MinionLife` on shields — where no
   ordering is derivable from the data at all. Those appear only once essence mods are in the ladder,
   which is the same wound as (2).

**What should replace the straddle rule — recommendation, not a decision.** Stop asking which tier a
roll is and compare the numbers. A `Member` currently means *"this group, at or better than tier
N"*; store instead the **threshold band** that tier N implies, and test the roll against it directly.
Every case in this census then answers without an annotation, without a tier ordinal, and without
caring whether GGG counts essence mods in its ladder. Details and worked cases in
[What to do instead](#what-to-do-instead).

---

## Scope: what "every in-scope Base Group" turned out to mean

A Base Group is item class + tag set, keyed exactly the way `mods_by_base.json` keys it — the map's
unit for one pool file. In scope: the four flask classes, and the five armour slots.

| item class | Base Groups | notes |
| --- | --- | --- |
| Life Flasks | 2 | `life_flask,…` and the `not_for_sale` variant (Divine / Eternal), identical pools |
| Mana Flasks | 2 | |
| Hybrid Flasks | 2 | |
| Utility Flasks | 2 | the second is the Expedition flask (`expedition_flask`) |
| Body Armours | 19 | 6 attribute combinations × plain / `top_tier_base_item_type` / `not_for_sale` |
| Boots | 27 | includes 4 Atlas base types and 1 `ward_armour` |
| Gloves | 26 | |
| Helmets | 23 | |
| Shields | 17 | |
| **total** | **120** | covering **525** distinct base items |

Six further tag sets exist and were excluded because no base in them is `release_state: released`:
`Kaom's Plate` (unreleased) and the five demigod bases (`unique_only`), which the map rules out
anyway.

Sizes, counting only the `prefix` and `suffix` pools (the map's two currencies; corrupted, delve and
influence pools are recognition-only):

| | Mod Groups | tier rows |
| --- | --- | --- |
| as `mods_by_base.json` lists them | 2 921 | 16 559 |
| minus the 3 Royale mods | 2 886 | 16 464 |
| plus every essence mod for the class | 4 235 | 23 678 |

Those are per-Base-Group counts summed over the 120 Base Groups, so the same Mod Group is counted
once per Base Group it appears in. Distinct mods in scope: 891.

### Trust in the derived display bands

`build-mod-pool.py`'s self-check 2 — the derived `display_min` / `display_max` must reproduce the
numbers in RePoE's own rendered `text` — was run over all 891 distinct in-scope mods:
**848 agree, 43 do not, 0 hit an unknown `index_handler`.** The generator raises `SystemExit` on an
unknown handler and this scope reaches far more of them than the jewel did, so the census extends
that table; the fact that the extension still reproduces 848 renderings is the evidence it is right.

The 43 misses are enumerated in the census output and are all benign for this question:

| class | n | what happens |
| --- | --- | --- |
| flag stat | 26 | a stat with `min = max = 1` that renders no number at all (`FlaskCurseImmunity1` → *"Removes Curses on use"*). Both tiers of a group carry it, so band comparison is unaffected |
| value rendered twice | 8 | the text prints one stat's value in two places (`FlaskChillFreezeImmunity*` → *"…Chill for (6-8) seconds… Freeze for (6-8) seconds"*) |
| literal in the string | 7 | the translation itself contains a number (`LocalLifeFlaskAdditionalLifeRecovery*` → *"…over 10 seconds"*; the two *"for each nearby Enemy, up to a maximum of 50%"* essence mods) |
| one placeholder, two stats | 1 | `ElementalDamageTakenWhileStationaryEssence1` carries two 5s and renders one |
| genuine handler mismatch | 1 | `BurningGroundWhileMovingEssence1` derives `2.5` where the text says `2500`. Essence-only, no partner in its group, cannot affect the census |

---

## 1. The overlap census

The predicate is `build-mod-pool.py` self-check 3's: two tiers overlap when **every** stat's display
band intersects the other's. The census tightens it with one extra condition the generator does not
apply and should — *the two tiers must render the same lines over the same stat ids*. Without that,
`BaseLocalDefences` reports 322 "identical range" pairs that are really *+(301-400) to Armour* versus
*+(301-400) to Evasion Rating*: different text, never confusable, and a false positive of the
generator's own test, which `zip`s the two tiers' stat arrays positionally without comparing ids.
Pairs that pass both conditions are called **confusable** below.

### Pool mods only

| variant | confusable pair instances | distinct (class, group, mod pair) | Base Groups affected |
| --- | --- | --- | --- |
| as `mods_by_base.json` lists them | 294 | 27 | 93 of 120 |
| minus the 3 Royale mods | **114** | **18** | **71 of 120** |

The 9 extra distinct pairs are entirely the work of **three mods that cannot spawn in the parent
game**: `MovementVelocity2Royale` *(15-25)%* (28 Base Groups), `IncreasedAttackSpeed2Royale`
*(8-15)%* (34), `IncreasedCastSpeed2Royale` *(8-15)%* (36). RePoE gives them no flag —
`is_essence_only: false`, `generation_weights: []`, and `spawn_weights: [{boots: 1000}, {default: 0}]`
identical in shape to a real mod. The only discriminator is the substring `Royale` in the mod id.
`IncreasedCastSpeed` exists as a glove and shield suffix group *solely* because of one of them.

The 18 distinct pairs that survive, with the per-Roll ambiguity they create:

| item class | Mod Group | tiers | bands | P(ambiguous roll \| group rolled) |
| --- | --- | --- | --- | --- |
| Utility Flasks | `FlaskBuffCurseEffect` | T2 / T3 | 52–59 / 48–52 | **6.5 %** |
| Utility Flasks | `FlaskBuffShockEffect` | T2 / T3 | 52–59 / 48–52 | **6.5 %** |
| Gloves | `ColdDamage` | T2 / T3 | (8-11)/(16-19) vs (6-9)/(13-16) | 4.2 % |
| Boots, Gloves, Helmets | `LocalEnergyShieldPercent` | T6 / T7 | 27–42 / 11–28 | 3.4 % |
| Boots, Gloves, Helmets | `LocalWardPercent` | T6 / T7 | 27–42 / 11–28 | 3.4 % |
| Body Armours, Shields | `LocalEnergyShieldPercent` | T7 / T8 | 27–42 / 11–28 | 3.0 % |
| Boots, Gloves | `LocalWard` | T2 / T3 | 52–69 / 36–52 | 1.6 % |
| Helmets | `LocalWard` | T3 / T4 | 52–69 / 36–52 | 1.4 % |
| Utility Flasks | `FlaskBuffChillFreezeDuration` | T2 / T3 | 52–59 / 48–52 | 1.1 % |
| Shields | `IncreasedAccuracy` | T4 / T5 | 100–165 / 50–100 | 0.6 % |
| Gloves, Helmets | `IncreasedAccuracy` | T5 / T6 | 100–165 / 50–100 | 0.5 % |

Rows share a Mod Group across item classes where the ladder is the same and only the tier numbers
shift, which is why 11 rows describe 18 (class, group) pairs. **Life, Mana and Hybrid Flasks have no
overlapping pair at all**; all six flask instances are on Utility Flasks.

The ambiguity column is exact, not sampled: it enumerates every integer roll of every tier, weights
tiers by `spawn_weight`, and reports the share that also falls inside another tier's band. `52` is
the only ambiguous value for the three flask groups; `IncreasedAccuracy` is ambiguous only at exactly
`100`.

### With essence mods folded in

The map requires every mod that can appear on the Base to be in the pool data, essence mods included,
because `UnrecognisedLine` `Halt`s. Doing that changes the picture:

| | value |
| --- | --- |
| distinct confusable pairs | **129** — 18 pool↔pool, 75 essence↔pool, 36 essence↔essence |
| Base Groups affected | 114 of 120 |
| worst Mod Groups (distinct pairs) | `IncreasedLife` on Boots / Gloves / Helmets: **14 each**; `MinionLife` on Shields: 8; `PhysicalDamage` on Gloves: 7; `LightningDamageAvoidance` on Body Armours / Shields: 5 each |

Per-Roll ambiguity, given the group rolled:

| item class | Mod Group | P(ambiguous) |
| --- | --- | --- |
| Boots, Gloves | `IncreasedLife` | **79 %** |
| Helmets | `IncreasedLife` | **71 %** |
| Shields | `MinionLife` | **60 %** |
| Gloves | `PhysicalDamage` | 33 % |
| Gloves `Dexterity`, Helmets `Intelligence` | | 16 % |
| all five classes | `Strength` / `Dexterity` / `Intelligence` | 11 % |
| all five classes | `IncreasedMana` | 8 % |

`IncreasedLife` is the clearest case. Essence of Greed's seven ranks grant
`IncreasedLifeEssence1…6` and `IncreasedLifeEssenceBootsGloves1`, whose bands
(+5-14, +15-30, +31-45, +46-60, +61-75, +76-90, +91-105) interleave with the normal boots ladder
(+3-9, +10-24, +25-39, +40-54, +55-69, +70-84, +85-99, +100-114, +115-129) at roughly half-tier
offsets. Four fifths of life rolls on a pair of boots therefore match two tiers.

**Essence-only ladders are worse still, and are exactly what the essence craft guarantees.** The
avoidance suffixes step in 1-point increments across 2-wide bands —
`ChanceToAvoidColdDamageEssence3…7` are (5-6), (6-7), (7-8), (8-9), (9-10)% — so **every** roll of a
middle rank matches two tiers. Their ambiguity share is 1.00. They contribute 0 to the weighted
column above only because essence mods have `spawn_weight: 0`; they are reached by using the essence,
not by rolling.

---

## 2. Overlaps that differ only by `required_ilvl`

**In the alteration pools: none.** Zero of the 18 distinct pairs have identical display bands, and
zero share a `required_ilvl`. Every pair is separated by at least 6 item levels.

**Once essence mods are in the ladder: 17 distinct pairs share a `required_ilvl`, and 0 of those have
identical bands.** All 17:

| item class | Mod Group | ilvl | tiers |
| --- | --- | --- | --- |
| all five armour classes | `Strength` | 82 | `Strength9` +(51-55) vs `StrengthEssence7_` +(51-58) |
| all five armour classes | `Dexterity` | 82 | `Dexterity9` +(51-55) vs `DexterityEssence7` +(51-58) |
| Body Armours, Boots, Gloves, Helmets, Shields | `Intelligence` | 82 | `Intelligence9` +(51-55) vs `IntelligenceEssence7` +(51-58) |
| Shields | `MinionLife` | 10 | `MinionLifeEssence2` (13-15)% vs `MinionLifeWeapon1` (13-17)% |
| Shields | `MinionLife` | 26 | `MinionLifeEssence3_` (16-18)% vs `MinionLifeWeapon2` (18-22)% |

(15 attribute pairs + 2 `MinionLife` = 17.) The essence mods come from Deafening Essence of Rage,
Sorrow and Spite respectively, and the `MinionLife` ones from Muttering and Weeping Essence of Fear;
`Strength9` / `Dexterity9` / `Intelligence9` are the ilvl-82 normal suffixes that sit at the same
level.

These are the pathological case the ticket asked about, and they exist. The generator's tier
numbering sorts by `-required_ilvl` then by `mod_id`, so `Strength9` becomes Tier 1 and
`StrengthEssence7_` Tier 2 **by alphabetical accident**. Nothing in the data justifies that order.

---

## 3. Essence-only mods against normal tiers

### The charted worry is false

`MovementVelocityEssence7` — Deafening Essence of Zeal, ilvl 82 — is a **flat 32 % increased Movement
Speed**. The normal boots ladder is 10 / 15 / 20 / 25 / 30 / 35 %, all flat single values. 32 %
collides with nothing. Better still, the other three Zeal ranks that produce movement speed grant the
*ordinary pool mods* `MovementVelocity3`, `4` and `5` — the same mod ids already in the pool, so they
add no tiers at all. The one remaining essence movement mod,
`MovementSpeedPerNearbyEnemyEssence1` (Essence of Desolation), renders a different line
(*"…for each nearby Enemy, up to a maximum of 50%"*) and is never confusable with it.

The same is true of the entire acceptance-test rule on the map,
`And[Movement Speed, Chaos Resistance T2] + Count(1)[Fire Res T2, Cold Res T2, Lightning Res T2]`:
`FireResistance`, `ColdResistance`, `LightningResistance` and `ChaosResistance` on boots have **no
overlapping bands and no essence-only mods** — every resistance essence rank grants an ordinary pool
mod (`ColdResist1…8`, `ChaosResist4…6`, …). The craft the map was designed around is clean.

### Where essences do collide

75 distinct essence↔pool pairs and 36 essence↔essence pairs, listed by the census. The pattern is
consistent: GGG built the essence ranks as their own ladder pitched *between* the normal tiers, so
essence bands straddle normal ones almost by construction. Examples:

| item class | Mod Group | essence tier | normal tier |
| --- | --- | --- | --- |
| Body Armours | `IncreasedMana` | `IncreasedManaEssence7` +(69-77), ilvl 82 | `IncreasedMana12` +(69-73), ilvl 81 |
| Boots | `IncreasedLife` | `IncreasedLifeEssence6` +(76-90), ilvl 74 | `IncreasedLife5` +(70-84), ilvl 30 |
| Body Armours | `BaseLocalDefences` | `LocalIncreasedEvasionRatingEssence7` +(390-475), ilvl 82 | `LocalIncreasedEvasionRating10` +(301-400), ilvl 69 |
| Gloves | `Dexterity` | `DexterityEssence7` +(51-58), ilvl 82 | `Dexterity10` +(56-60), ilvl 85 |

### The bigger essence problem: numbering

Because a group's tier numbers are *derived* from `required_ilvl` order, inserting essence mods into
the ladder renumbers the normal tiers below them:

| | value |
| --- | --- |
| (class, group) pairs that gain essence tiers | **32** |
| of those, pairs where at least one normal tier changes number | **32** |
| normal tier rows whose number changes | **241** |

Worst: `IncreasedMana` on all four of body armour / boots / gloves / helmets — one essence tier at
ilvl 82 above a 12-tier normal ladder renumbers **all 12**. `IncreasedLife` on helmets: 10 of 10.
`Strength` / `Dexterity` / `Intelligence`: 8 of 9, on every armour class.

And on the acceptance test's own Member: `MovementVelocity` on boots gains 2 essence tiers and
renumbers **5 of 6** normal tiers. *30 % increased Movement Speed* is Tier 2 in a pool file without
essence mods and Tier 4 in one with them. One of those is what the game prints.

---

## 4. Would `(Tier: N)` actually break the ties?

This is the part the ticket said matters most, and the answer has three layers.

**(a) For every overlap in the alteration pools: yes, in principle.** All 18 distinct pairs have
different `required_ilvl`, so any ladder ordered by item level assigns them different numbers, and the
annotation names exactly one. The flask craft's straddles are all resolvable this way.

**(b) For the 17 same-`required_ilvl` pairs: no.** Two mods at the same item level have no derivable
order. Our numbering breaks the tie alphabetically by `mod_id`; GGG's ladder breaks it however GGG
breaks it. The annotation will print one number, and we cannot know whether it means `Strength9` or
`StrengthEssence7_`. Worse than unresolved: if our alphabetical guess is the wrong way round, the
annotation *disagrees* on every ordinary Roll of a +Strength suffix, which is a `Halt`. These are
among the most common suffixes in the game.

**(c) And the tiebreak presupposes what it is supposed to establish.** `(Tier: N)` can only
disambiguate two tiers if the derived numbering already agrees with the game's numbering in the
ordinary, non-overlapping case. The census finds two independent reasons it may not:

- **Royale mods** sit in the in-scope pools with real weights and no flag. If GGG's ladder excludes
  them and ours includes them, three groups are off by one below the Royale mod's item level
  (`MovementVelocity1`, `IncreasedAttackSpeed1`).
- **Essence mods** renumber 241 tiers across 32 (class, group) pairs, per section 3.

`verdict.rs` handles the disagreement in the safe direction — `AnnotationDisagrees` is `halt_worthy()`
— so a wrong ladder halts rather than mis-Latches. That is the correct failure, and it still means the
app stops working on the first Roll that produces a renumbered tier.

**This is not settleable from the export.** RePoE exports mods and levels; the tier integer the client
prints is not in the files, it is computed by the client from whatever ladder the client uses. The
census can say the two candidate ladders differ, and by how much, but not which one is right. It needs
**one in-game capture** — copy a rare pair of boots carrying *30 % increased Movement Speed* and a
+Strength suffix with Advanced Mod Descriptions on, and read the numbers. That is a five-minute job on
the gaming PC and it should happen before the essence craft is built, not after.

Note also that #4's poedb cross-check does **not** transfer here, and the map's reasoning that it
verified "the method" rather than the base is only partly right. The jewel base has **no** Royale
mods, **no** essence-only mods, **no** `match_string` collisions and **no** group whose tiers render
different text — verified against the shipped `data/ghastly-eye-jewel.json`. Every one of the
generator's assumptions that armour breaks was vacuously true there. Choosing the two Base Groups to
hand-verify should therefore be deliberate: one hybrid-defence armour (to exercise the collisions) and
one essence-craftable armour (to exercise the numbering).

---

## 5. Mod Groups that share a rendered-text match string

This is the census's largest number and its actual blocker.

| | value |
| --- | --- |
| distinct colliding `(match_string, group pair)` | **53** |
| in-scope Base Groups with at least one collision | **120 of 120** |
| collision instances, by class | Boots 183 · Helmets 165 · Gloves 148 · Body Armours 124 · Shields 98 · Hybrid Flasks 12 · Mana Flasks 10 · Utility Flasks 10 · Life Flasks 8 |

`pool.rs::from_json` returns `DataError::AmbiguousMatchString` on the first one, and
`build-mod-pool.py` self-check 3 fails loudly. **Every flask and armour pool file would be rejected,
by both.**

The cause is structural. A hybrid defence mod renders *two* lines, and each of those lines is some
other group's only line:

```
LocalPhysicalDamageReductionRating       "# to Armour"
LocalBaseArmourAndEvasionRating          "# to Armour"  +  "# to Evasion Rating"
LocalBaseArmourAndEnergyShield           "# to Armour"  +  "# to maximum Energy Shield"
LocalBaseArmourAndLife                   "# to Armour"  +  "# to maximum Life"
```

The worst offenders, and the shape of each:

| `match_string` | groups sharing it | why |
| --- | --- | --- |
| `#% increased Stun and Block Recovery` | 9 groups | `StunRecovery` plus 8 hybrid `…AndStunRecovery` groups — stun recovery is bolted onto every defence-percent mod |
| `# to maximum Energy Shield` | 5 groups | flat ES, and 4 hybrids |
| `# to maximum Life` | 5 groups | `IncreasedLife` and 4 `…AndLife` hybrids |
| `# to Armour` | 4 groups | flat armour, and 3 hybrids |
| `# to Evasion Rating` | 4 groups | flat evasion, and 3 hybrids |
| `#% less Duration` | 5 flask immunity groups | every *"Immunity to X during Effect"* suffix carries the same drawback line |
| `#% reduced Amount Recovered` | 4 flask groups | same |
| `#% increased Armour` | 2 groups | `…Percent` and `…AndStunRecoveryPercent` |

**The fix is a matcher change, not a data change.** A single rendered line does not identify a Mod
Group on armour, and no pool file can make it. What *does* identify one is the **set** of lines under
one `{ Prefix Modifier … }` header — which is exactly what Advanced Mod Descriptions supplies. That
promotes the setting from "cross-check" to "load-bearing for identification", one step further than
the map's note that it "stops being optional". Without it, `+50 to Armour` on a body armour is
genuinely ambiguous between four groups and no amount of data fixes that.

### Two further generator refusals the census turned up

Both are separate failures from overlap and collision, and both would stop a pool file being
generated at all.

**Groups whose tiers do not render the same text.** `build_pool()` asserts that every tier of a group
produces the same `match_strings` list. It fails for **4** distinct (class, group) pairs in the
alteration pools and **15** once essence mods are in:

| class | group | signatures |
| --- | --- | --- |
| all four flask classes | `FlaskFullRechargeOnHit` | *"Gain # Charge…"* vs *"Gain # Charges…"* — **singular versus plural**, chosen by the rolled value |
| all five armour classes | `BaseLocalDefences` | *# to Armour* / *# to Evasion Rating* / *# to maximum Energy Shield* in one group |
| Gloves, Helmets | `Supported` | three unrelated socketed-gem lines |
| Boots | `MovementVelocity` | flat vs *"for each nearby Enemy"* |
| Gloves | `IncreasedAttackSpeed` | flat vs *"Attack and Cast Speed for each nearby Enemy"* |
| Shields | `SpellBlockPercentage` | unconditional vs *"while on Low Life"* |
| Boots | `ItemGrantsBuff` | two unrelated essence lines |

The pluralisation case is the interesting one: a group's `match_string` set is **value-dependent**, so
`match_lines` has to be a set of alternatives rather than one list per group.

**Unresolved match lines: 28, across 14 (class, group) pairs, all flasks.** `match_lines()` cannot
cover the mod's stat ids with any `stat_translations` entry, so `match_string` comes out `null` —
`FlaskChargesUsed`, `FlaskExtraMaxCharges`, `FlaskIncreasedHealingCharges`,
`FlaskEffectReducedDuration`, `FlaskIncreasedRecoveryReducedEffect`, `FlaskUtilityIncreasedDuration`.
Self-check 3 fails on each.

### A live bug found on the way: inverted display bands

**40 mods across 13 (class, group) pairs have `display_min > display_max`** — negated stats, where
RePoE's own `text` also reads backwards (`(59-52)% reduced Effect of Curses on you during Effect`).
All 40 are flask mods. `Band::contains` in `pool.rs` is `value >= self.min && value <= self.max`, so
an inverted band **accepts nothing**: every roll of those groups would produce `NoTierMatched` and
fail closed to a `Hit`.

This is not hypothetical and not new. **Three rows in the shipped
[`data/ghastly-eye-jewel.json`](../../data/ghastly-eye-jewel.json) already have it** —
`EnchantmentBlind` (`display_min: 30`, `display_max: 20`), `EnchantmentConsecratedGround` (25 / 15)
and `EnchantmentHinder` (35 / 25). Any Craft Session whose Target Mod is one of those three groups
Latches on the first Roll that produces the mod. Normalising the band at generation time
(`lo, hi = min(...), max(...)`) fixes all 43 rows.

---

## 6. Verdict on the charted rule, and what to do instead

### The rule survives

The map replaced blanket fail-closed with: every matching tier satisfies the Member's threshold →
`Hit`; none does → `Miss`; a genuine straddle falls to the annotation, failing closed to `Hit` when
absent. Against this census:

- It is **not** the case that the straddle is the common case in the alteration pools. Worst group in
  the whole flask and armour census is 6.5 % of rolls of one Utility Flask suffix, and 71 of 120 Base
  Groups have no overlap at all.
- The per-Member framing does most of the work by itself. Overlapping tiers are always *adjacent*, so
  a straddle needs the threshold to fall exactly between them; for any other threshold the "all
  match" or "none match" branch fires and the annotation is never consulted.
- The one place it does not survive intact is the essence craft, where `IncreasedLife` on boots is
  ambiguous on 79 % of rolls and the essence-only avoidance ladders are ambiguous on 100 % — and where
  the annotation, per section 4, is exactly the thing whose agreement cannot be assumed.

So: **keep it, and do not treat the annotation as the answer.**

### What to do instead

**Make a Member a numeric threshold rather than a tier ordinal.**

Today `Member = { group_id, tier_threshold }` and `assess()` derives a tier in order to compare
ordinals. Instead, when the rule is built, resolve the picked tier to its **display band** and store
that: `Member = { group_id, min_values: Vec<f64> }`. A rolled mod satisfies the Member when every
value is at least the corresponding `min_value`. Tier identity is never needed.

What this buys, case by case from this census:

| case | tier logic | threshold logic |
| --- | --- | --- |
| `Strength9` +(51-55) vs `StrengthEssence7_` +(51-58), both ilvl 82, roll 53 | straddle → annotation → cannot order them | both bands start at 51; the threshold is the same number either way. `53 >= 51` → `Hit`. **The ambiguity is irrelevant to the question asked** |
| `IncreasedLife6` +(85-99) picked, roll 90, also inside `IncreasedLifeEssence6` +(76-90) | straddle → annotation | `90 >= 85` → `Hit`. Which mod it is does not change that the human got the life they asked for |
| `FlaskBuffCurseEffect` T2 picked (52–59), roll 52, also inside T3 (48–52) | straddle → annotation | `52 >= 52` → `Hit` |
| essence avoidance ladder, every roll matches two ranks | straddle on every Roll | the picked rank's minimum is a number; compare to it |
| ladder renumbered by essence or Royale mods | wrong `derived`, `AnnotationDisagrees` → `Halt` | the threshold band is unchanged by renumbering. Numbering can only mislabel a log line |

It also restores what `verdict.rs`'s own doc comment wants: the annotation becomes a pure
`Diagnostic`, never load-bearing. `ManyTiersMatched` stays as a Diagnostic — it is still worth logging
that a roll was inside two bands — but it stops deciding a Verdict.

Three caveats, all decidable:

1. **Multi-stat mods need a rule.** *Adds (8-11) to (16-19) Cold Damage* is not a total order. "Every
   stat at least the threshold tier's `display_min`" is the natural reading and is what the tier
   comparison implied anyway.
2. **The picker still shows tiers**, because that is how humans talk about mods. The translation from
   picked tier to threshold band happens once, when the rule is built, and the persisted setup should
   store the band so a data refresh that renumbers tiers cannot silently move a threshold.
3. **It is slightly more fail-closed than tier logic**, in the direction `CONTEXT.md` mandates: a roll
   in a worse tier's band that clears the better tier's minimum reads as a `Hit`. That is the safe
   direction, and it is arguably also the correct one — the human asked for numbers.

### And, independently of the rule

These are what actually stand between the map and a working armour pool, in order:

1. **Identify a mod by its full line set, not one line.** 53 collisions on 120 of 120 Base Groups;
   nothing loads until this changes. Requires Advanced Mod Descriptions for grouping.
2. **Normalise inverted display bands at generation time.** 40 mods in scope, 3 live in the shipped
   jewel data.
3. **Exclude Royale mods by mod id.** 3 mods, 9 spurious overlaps, one entirely fictional Mod Group
   (`IncreasedCastSpeed` on gloves and shields).
4. **Settle tier numbering with one in-game capture** before building the essence craft — does GGG's
   ladder count essence mods? 241 tier numbers hang on the answer.
5. **Let `match_lines` hold alternatives.** Singular/plural (`FlaskFullRechargeOnHit`) and
   multi-signature groups (`BaseLocalDefences`, `Supported`) both need it; the generator's `assert`
   currently forbids it.
6. **Resolve or explicitly allow `null` match strings** for the 6 flask groups whose stats have no
   translation.

---

## Reproducing this

Upstream (~75 MB, deliberately not committed) is the same set `build-mod-pool.py`'s docstring fetches,
plus `essences.json`:

```
B=https://raw.githubusercontent.com/repoe-fork/repoe-fork.github.io/master
mkdir -p .upstream && cd .upstream
for f in mods.json mods_by_base.json base_items.json stat_translations.json essences.json; do
  curl -sLO "$B/data/$f"
done
curl -sLO "$B/version.txt"
```

Then:

```
UPSTREAM=.upstream OUT=/tmp/census python3 scripts/census-tier-overlaps.py
```

It prints the tables above and writes `/tmp/census.json` with every pair, collision, split group and
ambiguity figure, so any number here can be re-read rather than re-derived. Runtime is a few minutes,
almost all of it the exact ambiguity enumeration.

Provenance, matching `data/ghastly-eye-jewel.json`'s `source` block:

| field | value |
| --- | --- |
| primary | RePoE (`repoe-fork` export), read from `master` rather than Pages |
| primary_url | <https://repoe-fork.github.io/> — repo <https://github.com/repoe-fork/repoe> |
| game_version | **3.29.2.1** |
| files_used | `mods.json`, `mods_by_base.json`, `base_items.json`, `stat_translations.json`, `essences.json` |
| captured_at | 2026-08-06 |
| cross-checked against | nothing external — this census is the primary computation. No poedb scrape, per map #21 |

**The shipped pool is a patch behind.** `data/ghastly-eye-jewel.json` stamps `3.29.1.2.2`; upstream
now reads `3.29.2.1`. Nothing in this census depends on the difference, but it is the staleness check
[`mod-tier-data.md`](mod-tier-data.md) said should exist in CI and still does not.

---

## Open questions

- **Does the game's `(Tier: N)` ladder include essence-only mods?** The single most valuable unknown
  in this document. 241 derived tier numbers depend on it and no export answers it. One clipboard
  capture from a rare item carrying an essence-tier mod settles it.
- **Does it include Royale mods?** Same shape, much smaller stakes: 3 mods, 3 groups, one tier each.
- **How does GGG order two mods at the same `required_ilvl`?** Needed for the 17 same-level pairs.
  Threshold logic makes the answer unnecessary, which is the strongest argument for threshold logic.
- **Do the four corruption-only essences (Delirium, Horror, Hysteria, Insanity) behave differently in
  the pool?** They are flagged `is_corruption_only: true` in `essences.json` and their mods were
  included in this census's essence pass. Which essences may be applied to an already-Rare item is a
  separate question, asserted while charting and not verified here.
- **`Charms` and `Tinctures` are separate item classes** in `mods_by_base.json` and were not treated
  as flasks. If the picker is meant to offer them, they need their own census — `Tinctures` has one
  Base Group, `Charms` three.
