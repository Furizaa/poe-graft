# Essence tiers, the Rare-reroll rule, and the guaranteed mod per armour slot

Research for [#22](https://github.com/Furizaa/poe-graft/issues/22), on
[map #21](https://github.com/Furizaa/poe-graft/issues/21). Captured **2026-08-06**.

```
source
  primary          RePoE (repoe-fork export)
  primary_url      https://repoe-fork.github.io/
  primary_repo     https://github.com/repoe-fork/repoe
  game_version     3.29.2.1                     (upstream version.txt)
  files_used       essences.json, mods.json, mods_by_base.json, base_items.json, version.txt
  captured_at      2026-08-06
  cross_checked    https://poedb.tw/us/<essence name> — the in-game item description text
                   for one essence per tier, quoted verbatim below
  clipboard_fixtures  four independent real Alt+Ctrl+C captures of essence-crafted items
                      (PathOfBuilding, xiletrade, poe-item-parser, poe-recomb), cited below
```

**The export is newer than the committed data.** `version.txt` in this capture reads
**`3.29.2.1`**; [`data/ghastly-eye-jewel.json`](../../data/ghastly-eye-jewel.json) pins
**`3.29.1.2.2`** (captured 2026-08-04). Nothing in this document depends on which of the two you
read — I did not diff the whole export — but the next capture will move the jewel pool's stamp, so
the pipeline ticket should expect the regenerated `ghastly-eye-jewel.json` to change its
`source.game_version` and should keep the
`poe-tool-dev/latest-patch-version` staleness check that
[`docs/research/mod-tier-data.md`](mod-tier-data.md) asked for.

---

## Verdict

**The owner's assertion is correct, with a sharp boundary.** An Essence can be applied to an
already-Rare item **iff its tier is Screaming (5) or higher**. Screaming, Shrieking and Deafening,
plus all five tier-8 essences, read *"left click a normal **or rare** item"*. Whispering,
Muttering, Weeping and Wailing read *"left click a **normal** item"* — they cannot touch a Rare, so
they are out of scope for this map exactly as [#21](https://github.com/Furizaa/poe-graft/issues/21)
said. That is **65 in-scope essences**: 20 families × 3 tiers, plus 5 tier-8 specials.

**No Scour is needed and one Trigger Press still drives one currency.** The charted risk is retired
against a primary source.

**Three findings change work the pipeline ticket was going to do:**

1. **Essence mods are not in `mods_by_base.json` at all** — zero of the 12 558 mod ids that file
   references anywhere are `is_essence_only`. The only join from an essence to its mod is
   `essences.json`'s `mods` map, keyed by the item-class *display* name. `mods.json` carries the
   mod, and it carries the flag `is_essence_only` that names the category.
2. **The game prints no `(Tier: N)` for an essence-only mod**, and on the five armour slots all 118
   of them are named `"Essences"` (every prefix) or `"of the Essence"` (every suffix). So the affix
   name identifies *that a mod came from an essence* but never *which* essence — and the tier
   annotation the map planned to use as a tie-breaker is always absent for exactly these mods.
3. **Essence-mod ranges overlap normal tier ranges of the same Mod Group.** Every one of the 28
   boots Base Groups, 27 gloves, 24 helmets, 21 body armours and 17 of 18 shields has at least one
   essence mod whose numeric range overlaps a normal-pool tier of the same group and the same stat.
   Map #1's "roll → tier is unambiguous" property does **not** survive contact with essences, which
   makes #21's per-Member straddle rule load-bearing rather than theoretical.

And one recommendation that follows from (2) and (3): **do not merge essence mods into a Mod
Group's `tiers[]` array.** See [Why essence mods must not join the tier ladder](#why-essence-mods-must-not-join-the-tier-ladder).

---

## The tier list, and the Rare-reroll rule per tier

`essences.json` gives each essence a `level` 0–8. Level maps one-to-one onto the name prefix, and
the Rare rule falls exactly on the level-4/level-5 boundary. The rule itself is **not a data-file
field** — it is the item's description text, so each row below is quoted from the in-game
description as poedb reproduces it, one essence sampled per tier.

| level | name prefix | count | can be applied to | verbatim description line | in scope |
| --- | --- | --- | --- | --- | --- |
| 1 | Whispering | 4 | Normal only | "Right click this item then left click a **normal item** to apply it." + "Properties restricted to level 35 and below" | no |
| 2 | Muttering | 8 | Normal only | "…left click a **normal item** to apply it." + "Properties restricted to level 45 and below" | no |
| 3 | Weeping | 12 | Normal only | "Upgrades a normal item to rare with one guaranteed property" / "…left click a **normal item**…" + "Properties restricted to level 60 and below" | no |
| 4 | Wailing | 16 | Normal only | "…left click a **normal item** to apply it." + "restricted to level 75 and below" | no |
| 5 | **Screaming** | 20 | **Normal or Rare** | "Upgrades a normal item to rare **or reforges a rare item**, guaranteeing one property" / "…left click a **normal or rare item** to apply it." | **yes** |
| 6 | **Shrieking** | 20 | **Normal or Rare** | "Upgrades a normal item to rare **or reforges a rare item**, guaranteeing one property" / "…left click a **normal or rare item** to apply it." | **yes** |
| 7 | **Deafening** | 20 | **Normal or Rare** | "Upgrades a normal item to rare **or reforges a rare item**, guaranteeing one property" / "…left click a **normal or rare item** to apply it." | **yes** |
| 8 | (no prefix) | 5 | **Normal or Rare** | "Upgrades a normal item to rare **or reforges a rare item**, guaranteeing one property" / "…left click a **normal or rare item** to apply it." | **yes** |
| 0 | Remnant of Corruption | 1 | *not an essence application* | it corrupts an essence, not an item | no |

Sources, one page per row:
[Whispering](https://poedb.tw/us/Whispering_Essence_of_Greed),
[Muttering](https://poedb.tw/us/Muttering_Essence_of_Greed),
[Weeping](https://poedb.tw/us/Weeping_Essence_of_Greed),
[Wailing](https://poedb.tw/us/Wailing_Essence_of_Zeal),
[Screaming](https://poedb.tw/us/Screaming_Essence_of_Zeal),
[Shrieking](https://poedb.tw/us/Shrieking_Essence_of_Greed),
[Deafening](https://poedb.tw/us/Deafening_Essence_of_Greed),
[tier 8 — Hysteria](https://poedb.tw/us/Essence_of_Hysteria),
[tier 8 — Horror](https://poedb.tw/us/Essence_of_Horror),
[tier 8 — Desolation](https://poedb.tw/us/Essence_of_Desolation).

**An independent corroboration from the data file itself**, which is why I trust the boundary
rather than only the six pages I read: `essences.json` sets `item_level_restriction` to a number
(35 / 45 / 60 / 75) for **exactly** levels 1–4 and to `null` for **exactly** levels 5–8. The field
tracks the same split as the description text, from a different source, with no exceptions in 106
rows.

Two honest caveats on that table:

- I quoted **Insanity** and **Delirium** from the shared tier-8 pattern rather than from their own
  pages; I read Hysteria, Horror and Desolation. All five sit at `level: 8` with
  `item_level_restriction: null`, and the four corruption-only ones share
  `type: {tier: 6, is_corruption_only: true}`. I would call this near-certain but not
  independently read.
- **What "Properties restricted to level N and below" means** is ambiguous in the item text: it
  could cap the *item's* level or cap the *mod* levels rolled. poedb's own summary reads it as the
  former. It does not matter for this app — the line is absent on every in-scope tier — so I did
  not chase it.

### Which essence families exist at which tier

A family appears from a fixed tier upward, recorded as `type.tier` in `essences.json`. All 20
families exist at Screaming and above, which is why the in-scope set is a clean 20 × 3.

| `type.tier` | families | first tier available |
| --- | --- | --- |
| 1 | Greed, Contempt, Hatred, Woe | Whispering |
| 2 | Anger, Fear, Sorrow, Torment | Muttering |
| 3 | Doubt, Rage, Suffering, Wrath | Weeping |
| 4 | Anguish, Loathing, Spite, **Zeal** | Wailing |
| 5 | Dread, Envy, Misery, Scorn | Screaming |
| 6 | Delirium, Horror, Hysteria, Insanity (corruption-only), Desolation | tier 8 only |

---

## What an essence does to a Rare, and what that means for the app

Established from the description text above:

- It **reforges** the Rare: every explicit mod is removed and the item is rolled again, with one
  mod guaranteed. The item stays Rare. So the roll cycle in `crates/core/src/cycle.rs` needs no
  new concept — it is the same "one currency, one reroll, then Read" shape as an Alteration.
- **One press, one currency.** No Scour, no Transmute, no Regal. Apply Mode holds.
- The guaranteed mod is an **ordinary mod on the item**, in an ordinary prefix or suffix slot, with
  an ordinary rendered line. #21's decision that the guaranteed mod is "an ordinary Member with no
  special treatment" is consistent with the data: for 53 of the 171 distinct guaranteed mods on the
  five armour slots the mod *is* a normal-pool mod that an alteration could have produced anyway.

Not established, and deliberately left as recorded unknowns — see
[Open questions](#open-questions): how many affixes an essence-applied Rare ends up with, and
whether the guaranteed mod respects the item's item level.

---

## Where the data lives, and how to join it

Three files, and the join is short enough to state completely.

### `essences.json` — the essence → item-class → mod id map

106 entries keyed by the currency's metadata id. One entry:

```json
"Metadata/Items/Currency/CurrencyEssenceZeal4": {
  "name": "Deafening Essence of Zeal",
  "level": 7,
  "type": { "tier": 4, "is_corruption_only": false },
  "item_level_restriction": null,
  "spawn_level_min": 0,
  "mods": {
    "Boots": "MovementVelocityEssence7",
    "Gloves": "IncreasedAttackSpeedEssenceGloves7",
    "Helmet": "WarcrySpeedEssence4_",
    "Body Armour": "SummonTotemCastSpeedEssence4",
    "Shield": "StunRecoveryEssence7",
    "…": "… 17 more item classes …"
  }
}
```

`mods` has **22 keys**, one per item class the essence can guarantee something on:
`Amulet`, `Belt`, `Body Armour`, `Boots`, `Bow`, `Claw`, `Dagger`, `Gloves`, `Helmet`,
`One Hand Axe`, `One Hand Mace`, `One Hand Sword`, `Quiver`, `Ring`, `Sceptre`, `Shield`, `Staff`,
`Thrusting One Hand Sword`, `Two Hand Axe`, `Two Hand Mace`, `Two Hand Sword`, `Wand`.

**The keys join to `base_items.json`'s `item_class` verbatim** — singular `Helmet`, two-word
`Body Armour`. That is the pipeline's lookup: pick the Base → read `base_items[base_id].item_class`
→ index `essences.json[essence].mods[item_class]`. (Note this is *not* the same string as
`mods_by_base.json`'s top-level key, which is plural: `Helmets`, `Body Armours`, `Shields`. Both
appear in this project already.) Confirmed base counts in `base_items.json`: Boots 83, Gloves 80,
Helmet 96, Body Armour 124, Shield 98.

Only `Remnant of Corruption` has `null` values in `mods` — for all 22 classes, because it is not
applied to an item at all. Skip `level: 0`.

### `mods.json` — the mod itself, and the `is_essence_only` flag

Join the mod id straight into `mods.json`. It is an ordinary mod record — the same shape
`build-mod-pool.py` already consumes — with two fields that matter here:

```json
"MovementVelocityEssence7": {
  "domain": "item",
  "generation_type": "prefix",
  "groups": ["MovementVelocity"],
  "is_essence_only": true,
  "name": "Essences",
  "required_level": 82,
  "spawn_weights": [{ "tag": "default", "weight": 0 }],
  "stats": [{ "id": "base_movement_velocity_+%", "min": 32, "max": 32 }],
  "text": "32% increased Movement Speed"
}
```

- **`is_essence_only: true`** is the category flag. 462 mods in the export carry it, all
  `domain: item`; 460 of them have `spawn_weights` exactly `[{default: 0}]` and 2 have `[]`.
  Meaning: **an essence-only mod has no spawn weight anywhere**, so no amount of tag resolution
  will ever find it. That is the whole reason it is invisible to `mods_by_base.json`.
- **`required_level`** is the only tier-like number these mods carry. For the in-scope tiers it is
  uniform: **58** for Screaming, **74** for Shrieking, **82** for Deafening, **63** for all five
  tier-8 essences. Exactly three exceptions across the five armour slots, all the same family:
  `ChanceToAvoidFreezeEssence5/6/7` (Deafening/Shrieking/Screaming Essence of Suffering, on boots
  and helmets) sit at `required_level: 1`.
- `stats[].min/max` are **raw stat units**, exactly as for normal mods, so
  `build-mod-pool.py`'s `index_handlers` / `display_min` / `display_max` machinery applies
  unchanged. None of the 118 essence-only mods on the five armour slots renders as more than one
  line; **seven of them carry a `0–0` stat that renders nothing** (`AttackerTakesDamageEssence5/6/7`,
  `ChanceToAvoidFreezeEssence5/6/7`, `BurningGroundWhileMovingEssence1`), which is the case
  `match_lines` already handles for `ChanceToAvoidFreezeAndChill` on the jewel.
- **`name` is the affix name, and for essence-only mods it is a literal marker.** Across all 462
  `is_essence_only` mods in the export: 215 prefixes named `Essences`, 234 suffixes named
  `of the Essence`, and 13 stragglers (6 with an empty name, 4 with `generation_type: unique`, and 3
  crafted-bench-looking prefixes `Marchioness's` / `Duchess's` / `Queen's`). **None of the
  stragglers is reachable from an in-scope essence on the five armour slots** — there the split is
  exactly 78 `Essences` prefixes and 115 `of the Essence` suffixes, no exceptions.
  Conversely, 11 mods that are *not* `is_essence_only` also carry those names — all of them
  `*Inverted` mods (`AddedFireDamageEssence7Inverted`, `DamageCannotBeReflectedPercentEssence1Inverted`,
  …). Every one has `spawn_weights: [{default: 0}]` and none appears in any `mods_by_base.json` pool
  for Boots, Gloves, Helmets, Body Armours or Shields, so on the five in-scope slots the affix-name
  signal is clean.

### `mods_by_base.json` — where they are **not**

Checked exhaustively: of the **12 558** distinct mod ids referenced anywhere in
`mods_by_base.json` — across every item class and every pool (`prefix`, `suffix`, `corrupted`,
`delve_*`, the influence pools, `scourge_*`, the eldritch implicits) — **exactly 0** are
`is_essence_only`. There is no `essence` pool key. So:

> **`mods_by_base.json` cannot tell you which essence mods a base can carry. `essences.json` is the
> only join, and it joins by item class, not by base tag set.**

That is a genuine shape difference from everything `build-mod-pool.py` does today, and it has a
consequence: because an essence mod is attributed to an **item class**, every base in the class
gets the same 65 essence mods regardless of its tag set. There is nothing base-specific to resolve.

### The join, in the shape `build-mod-pool.py` would write it

```python
ITEM_CLASS = base_items[BASE_ID]['item_class']          # e.g. 'Boots'
essence_pool = []
for cid, ess in essences.items():
    if ess['level'] < 5:                                # Screaming+ only: Rare-reroll capable
        continue
    mid = ess['mods'][ITEM_CLASS]
    m = mods[mid]                                       # ordinary mods.json record
    essence_pool.append({
        'essence_id': cid,
        'essence_name': ess['name'],                    # 'Deafening Essence of Zeal'
        'essence_level': ess['level'],                  # 5 | 6 | 7 | 8
        'mod_id': mid,
        'group': m['groups'][0],
        'generation_type': m['generation_type'],
        'affix_name': m['name'],                        # 'Essences' | 'of the Essence' | a real name
        'essence_only': bool(m['is_essence_only']),
        'required_ilvl': m['required_level'],
        'stats': m['stats'],                            # raw units; run index_handlers as today
        'text': m['text'],
    })
```

Counts that fall out of it, per armour slot, for the 65 in-scope essences:

| slot | distinct guaranteed mods | essence-only (new to the pool) | already in the normal pool |
| --- | --- | --- | --- |
| Boots | 65 | 43 | 22 |
| Gloves | 65 | 34 | 31 |
| Helmet | 65 | 45 | 20 |
| Body Armour | 65 | 36 | 29 |
| Shield | 65 | 35 | 30 |
| **union of the five** | **171** | **118** (48 prefix, 70 suffix) | **53** |

Each of the 65 in-scope essences maps to a *distinct* mod on each slot — no two in-scope essences
grant the same mod on the same item class. And all 53 normal-pool guaranteed mods are already
present in that class's `mods_by_base.json` pools, so they need no new data at all: **the parsing
gap is exactly the 118 essence-only mods.**

---

## The essence × item-class → guaranteed-mod table

All 65 in-scope essences across the five armour slots, generated from `essences.json` joined into
`mods.json`. `*` marks `is_essence_only: true`; `(P)`/`(S)` in the group line is prefix/suffix.
Values are RePoE's own rendered `text`, i.e. display units. Families are ordered by the tier they
first become available at, then alphabetically — the same order as the family table above.

### Essence of Contempt

Mod Group per slot: Boots → `AttackerTakesDamageNoRange` (P); Gloves → `PhysicalDamage` (P); Helmet → `AttackerTakesDamageNoRange` (P); Body Armour → `AttackerTakesDamageNoRange` (P); Shield → `AttackerTakesDamageNoRange` (P).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `AttackerTakesDamageEssence5`*<br>Reflects (51-100) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence6`*<br>Reflects (101-150) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence7`*<br>Reflects (151-200) Physical Damage to Melee Attackers |
| **Gloves** | `AddedPhysicalDamageEssenceGlovesQuiver5`*<br>Adds (4-5) to (8-9) Physical Damage to Attacks | `AddedPhysicalDamageEssenceGlovesQuiver6`*<br>Adds (5-6) to (9-10) Physical Damage to Attacks | `AddedPhysicalDamageEssenceGlovesQuiver7`*<br>Adds (6-7) to (10-11) Physical Damage to Attacks |
| **Helmet** | `AttackerTakesDamageEssence5`*<br>Reflects (51-100) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence6`*<br>Reflects (101-150) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence7`*<br>Reflects (151-200) Physical Damage to Melee Attackers |
| **Body Armour** | `AttackerTakesDamageEssence5`*<br>Reflects (51-100) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence6`*<br>Reflects (101-150) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence7`*<br>Reflects (151-200) Physical Damage to Melee Attackers |
| **Shield** | `AttackerTakesDamageEssence5`*<br>Reflects (51-100) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence6`*<br>Reflects (101-150) Physical Damage to Melee Attackers | `AttackerTakesDamageEssence7`*<br>Reflects (151-200) Physical Damage to Melee Attackers |

### Essence of Greed

Mod Group per slot: Boots → `IncreasedLife` (P); Gloves → `IncreasedLife` (P); Helmet → `IncreasedLife` (P); Body Armour → `IncreasedLife` (P); Shield → `IncreasedLife` (P).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `IncreasedLifeEssence5`*<br>+(61-75) to maximum Life | `IncreasedLifeEssence6`*<br>+(76-90) to maximum Life | `IncreasedLifeEssenceBootsGloves1`*<br>+(91-105) to maximum Life |
| **Gloves** | `IncreasedLifeEssence5`*<br>+(61-75) to maximum Life | `IncreasedLifeEssence6`*<br>+(76-90) to maximum Life | `IncreasedLifeEssenceBootsGloves1`*<br>+(91-105) to maximum Life |
| **Helmet** | `IncreasedLifeEssence5`*<br>+(61-75) to maximum Life | `IncreasedLifeEssence6`*<br>+(76-90) to maximum Life | `IncreasedLifeEssenceBootsGloves1`*<br>+(91-105) to maximum Life |
| **Body Armour** | `IncreasedLife9`<br>+(130-144) to maximum Life | `IncreasedLife10`<br>+(145-159) to maximum Life | `IncreasedLife11`<br>+(160-174) to maximum Life |
| **Shield** | `IncreasedLife7`<br>+(100-114) to maximum Life | `IncreasedLife8`<br>+(115-129) to maximum Life | `IncreasedLife9`<br>+(130-144) to maximum Life |

### Essence of Hatred

Mod Group per slot: Boots → `ColdResistance` (S); Gloves → `ColdResistance` (S); Helmet → `ColdResistance` (S); Body Armour → `ColdResistance` (S); Shield → `ColdResistance` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `ColdResist6`<br>+(36-41)% to Cold Resistance | `ColdResist7`<br>+(42-45)% to Cold Resistance | `ColdResist8`<br>+(46-48)% to Cold Resistance |
| **Gloves** | `ColdResist6`<br>+(36-41)% to Cold Resistance | `ColdResist7`<br>+(42-45)% to Cold Resistance | `ColdResist8`<br>+(46-48)% to Cold Resistance |
| **Helmet** | `ColdResist6`<br>+(36-41)% to Cold Resistance | `ColdResist7`<br>+(42-45)% to Cold Resistance | `ColdResist8`<br>+(46-48)% to Cold Resistance |
| **Body Armour** | `ColdResist6`<br>+(36-41)% to Cold Resistance | `ColdResist7`<br>+(42-45)% to Cold Resistance | `ColdResist8`<br>+(46-48)% to Cold Resistance |
| **Shield** | `ColdResist6`<br>+(36-41)% to Cold Resistance | `ColdResist7`<br>+(42-45)% to Cold Resistance | `ColdResist8`<br>+(46-48)% to Cold Resistance |

### Essence of Woe

Mod Group per slot: Boots → `BaseLocalDefences` (P); Gloves → `BaseLocalDefences` (P); Helmet → `BaseLocalDefences` (P); Body Armour → `BaseLocalDefences` (P); Shield → `BaseLocalDefences` (P).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `LocalIncreasedEnergyShieldEssenceBootsGloves5`*<br>+(27-32) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceBootsGloves6`*<br>+(28-35) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceBootsGloves7`*<br>+(38-45) to maximum Energy Shield |
| **Gloves** | `LocalIncreasedEnergyShieldEssenceBootsGloves5`*<br>+(27-32) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceBootsGloves6`*<br>+(28-35) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceBootsGloves7`*<br>+(38-45) to maximum Energy Shield |
| **Helmet** | `LocalIncreasedEnergyShieldEssenceHelm5`*<br>+(39-45) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceHelm6`*<br>+(46-51) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceHelm7`*<br>+(52-58) to maximum Energy Shield |
| **Body Armour** | `LocalIncreasedEnergyShield8`<br>+(50-61) to maximum Energy Shield | `LocalIncreasedEnergyShield10`<br>+(77-90) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceChest7__`*<br>+(88-95) to maximum Energy Shield |
| **Shield** | `LocalIncreasedEnergyShieldEssenceShield5`*<br>+(50-59) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceShield6`*<br>+(60-69) to maximum Energy Shield | `LocalIncreasedEnergyShieldEssenceShield7`*<br>+(75-85) to maximum Energy Shield |

### Essence of Anger

Mod Group per slot: Boots → `FireResistance` (S); Gloves → `FireResistance` (S); Helmet → `FireResistance` (S); Body Armour → `FireResistance` (S); Shield → `FireResistance` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `FireResist6`<br>+(36-41)% to Fire Resistance | `FireResist7`<br>+(42-45)% to Fire Resistance | `FireResist8`<br>+(46-48)% to Fire Resistance |
| **Gloves** | `FireResist6`<br>+(36-41)% to Fire Resistance | `FireResist7`<br>+(42-45)% to Fire Resistance | `FireResist8`<br>+(46-48)% to Fire Resistance |
| **Helmet** | `FireResist6`<br>+(36-41)% to Fire Resistance | `FireResist7`<br>+(42-45)% to Fire Resistance | `FireResist8`<br>+(46-48)% to Fire Resistance |
| **Body Armour** | `FireResist6`<br>+(36-41)% to Fire Resistance | `FireResist7`<br>+(42-45)% to Fire Resistance | `FireResist8`<br>+(46-48)% to Fire Resistance |
| **Shield** | `FireResist6`<br>+(36-41)% to Fire Resistance | `FireResist7`<br>+(42-45)% to Fire Resistance | `FireResist8`<br>+(46-48)% to Fire Resistance |

### Essence of Fear

Mod Group per slot: Boots → `MinionLife` (S); Gloves → `MinionDamage` (P); Helmet → `MinionDamage` (P); Body Armour → `MinionLife` (S); Shield → `MinionLife` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `MinionLifeEssence5`*<br>Minions have (22-24)% increased maximum Life | `MinionLifeEssence6`*<br>Minions have (25-27)% increased maximum Life | `MinionLifeEssence7`*<br>Minions have (28-30)% increased maximum Life |
| **Gloves** | `MinionDamageGlovesEssence5`*<br>Minions deal (22-24)% increased Damage | `MinionDamageGlovesEssence6___`*<br>Minions deal (25-27)% increased Damage | `MinionDamageGlovesEssence7`*<br>Minions deal (28-30)% increased Damage |
| **Helmet** | `MinionDamageGlovesEssence5`*<br>Minions deal (22-24)% increased Damage | `MinionDamageGlovesEssence6___`*<br>Minions deal (25-27)% increased Damage | `MinionDamageGlovesEssence7`*<br>Minions deal (28-30)% increased Damage |
| **Body Armour** | `MinionLifeEssence5`*<br>Minions have (22-24)% increased maximum Life | `MinionLifeEssence6`*<br>Minions have (25-27)% increased maximum Life | `MinionLifeEssence7`*<br>Minions have (28-30)% increased maximum Life |
| **Shield** | `MinionLifeEssence5`*<br>Minions have (22-24)% increased maximum Life | `MinionLifeEssence6`*<br>Minions have (25-27)% increased maximum Life | `MinionLifeEssence7`*<br>Minions have (28-30)% increased maximum Life |

### Essence of Sorrow

Mod Group per slot: Boots → `Dexterity` (S); Gloves → `Dexterity` (S); Helmet → `Dexterity` (S); Body Armour → `Dexterity` (S); Shield → `Dexterity` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `Dexterity6`<br>+(33-37) to Dexterity | `Dexterity8`<br>+(43-50) to Dexterity | `DexterityEssence7`*<br>+(51-58) to Dexterity |
| **Gloves** | `Dexterity6`<br>+(33-37) to Dexterity | `Dexterity8`<br>+(43-50) to Dexterity | `DexterityEssence7`*<br>+(51-58) to Dexterity |
| **Helmet** | `Dexterity6`<br>+(33-37) to Dexterity | `Dexterity8`<br>+(43-50) to Dexterity | `DexterityEssence7`*<br>+(51-58) to Dexterity |
| **Body Armour** | `Dexterity6`<br>+(33-37) to Dexterity | `Dexterity8`<br>+(43-50) to Dexterity | `DexterityEssence7`*<br>+(51-58) to Dexterity |
| **Shield** | `Dexterity6`<br>+(33-37) to Dexterity | `Dexterity8`<br>+(43-50) to Dexterity | `DexterityEssence7`*<br>+(51-58) to Dexterity |

### Essence of Torment

Mod Group per slot: Boots → `ReducedShockChance` (S); Gloves → `LightningDamage` (P); Helmet → `ReducedShockChance` (S); Body Armour → `LightningDamageAvoidance` (S); Shield → `LightningDamageAvoidance` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `ChanceToAvoidShockEssence5`*<br>(47-50)% chance to Avoid being Shocked | `ChanceToAvoidShockEssence6`*<br>(51-55)% chance to Avoid being Shocked | `ChanceToAvoidShockEssence7`*<br>(56-60)% chance to Avoid being Shocked |
| **Gloves** | `AddedLightningDamage4`<br>Adds (1-2) to (27-28) Lightning Damage to Attacks | `AddedLightningDamage5`<br>Adds (1-3) to (33-34) Lightning Damage to Attacks | `AddedLightningDamage6`<br>Adds (1-4) to (40-43) Lightning Damage to Attacks |
| **Helmet** | `ChanceToAvoidShockEssence5`*<br>(47-50)% chance to Avoid being Shocked | `ChanceToAvoidShockEssence6`*<br>(51-55)% chance to Avoid being Shocked | `ChanceToAvoidShockEssence7`*<br>(56-60)% chance to Avoid being Shocked |
| **Body Armour** | `ChanceToAvoidLightningDamageEssence5`*<br>(7-8)% chance to Avoid Lightning Damage from Hits | `ChanceToAvoidLightningDamageEssence6`*<br>(8-9)% chance to Avoid Lightning Damage from Hits | `ChanceToAvoidLightningDamageEssence7`*<br>(9-10)% chance to Avoid Lightning Damage from Hits |
| **Shield** | `ChanceToAvoidLightningDamageEssence5`*<br>(7-8)% chance to Avoid Lightning Damage from Hits | `ChanceToAvoidLightningDamageEssence6`*<br>(8-9)% chance to Avoid Lightning Damage from Hits | `ChanceToAvoidLightningDamageEssence7`*<br>(9-10)% chance to Avoid Lightning Damage from Hits |

### Essence of Doubt

Mod Group per slot: Boots → `BaseLocalDefences` (P); Gloves → `BaseLocalDefences` (P); Helmet → `BaseLocalDefences` (P); Body Armour → `BaseLocalDefences` (P); Shield → `BaseLocalDefences` (P).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `LocalIncreasedEvasionRatingEssenceGlovesBoots5`*<br>+(91-105) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceGlovesBoots6`*<br>+(106-120) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceGlovesBoots7`*<br>+(121-135) to Evasion Rating |
| **Gloves** | `LocalIncreasedEvasionRatingEssenceGlovesBoots5`*<br>+(91-105) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceGlovesBoots6`*<br>+(106-120) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceGlovesBoots7`*<br>+(121-135) to Evasion Rating |
| **Helmet** | `LocalIncreasedEvasionRatingEssenceHelm5`*<br>+(121-140) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceHelm6`*<br>+(141-160) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceHelm7`*<br>+(161-180) to Evasion Rating |
| **Body Armour** | `LocalIncreasedEvasionRating8`<br>+(151-200) to Evasion Rating | `LocalIncreasedEvasionRating10`<br>+(301-400) to Evasion Rating | `LocalIncreasedEvasionRatingEssence7`*<br>+(390-475) to Evasion Rating |
| **Shield** | `LocalIncreasedEvasionRatingEssenceShield5`*<br>+(151-225) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceShield6`*<br>+(226-300) to Evasion Rating | `LocalIncreasedEvasionRatingEssenceShield7____`*<br>+(301-375) to Evasion Rating |

### Essence of Rage

Mod Group per slot: Boots → `Strength` (S); Gloves → `Strength` (S); Helmet → `Strength` (S); Body Armour → `Strength` (S); Shield → `Strength` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `Strength6`<br>+(33-37) to Strength | `Strength8`<br>+(43-50) to Strength | `StrengthEssence7_`*<br>+(51-58) to Strength |
| **Gloves** | `Strength6`<br>+(33-37) to Strength | `Strength8`<br>+(43-50) to Strength | `StrengthEssence7_`*<br>+(51-58) to Strength |
| **Helmet** | `Strength6`<br>+(33-37) to Strength | `Strength8`<br>+(43-50) to Strength | `StrengthEssence7_`*<br>+(51-58) to Strength |
| **Body Armour** | `Strength6`<br>+(33-37) to Strength | `Strength8`<br>+(43-50) to Strength | `StrengthEssence7_`*<br>+(51-58) to Strength |
| **Shield** | `Strength6`<br>+(33-37) to Strength | `Strength8`<br>+(43-50) to Strength | `StrengthEssence7_`*<br>+(51-58) to Strength |

### Essence of Suffering

Mod Group per slot: Boots → `ChanceToAvoidFreezeAndChill` (S); Gloves → `ColdDamage` (P); Helmet → `ChanceToAvoidFreezeAndChill` (S); Body Armour → `ColdDamageAvoidance` (S); Shield → `ColdDamageAvoidance` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `ChanceToAvoidFreezeEssence5`*<br>(47-50)% chance to Avoid being Frozen | `ChanceToAvoidFreezeEssence6`*<br>(51-55)% chance to Avoid being Frozen | `ChanceToAvoidFreezeEssence7`*<br>(56-60)% chance to Avoid being Frozen |
| **Gloves** | `AddedColdDamage4`<br>Adds (6-9) to (13-16) Cold Damage to Attacks | `AddedColdDamage5`<br>Adds (8-11) to (16-19) Cold Damage to Attacks | `AddedColdDamage6`<br>Adds (10-13) to (20-24) Cold Damage to Attacks |
| **Helmet** | `ChanceToAvoidFreezeEssence5`*<br>(47-50)% chance to Avoid being Frozen | `ChanceToAvoidFreezeEssence6`*<br>(51-55)% chance to Avoid being Frozen | `ChanceToAvoidFreezeEssence7`*<br>(56-60)% chance to Avoid being Frozen |
| **Body Armour** | `ChanceToAvoidColdDamageEssence5`*<br>(7-8)% chance to Avoid Cold Damage from Hits | `ChanceToAvoidColdDamageEssence6`*<br>(8-9)% chance to Avoid Cold Damage from Hits | `ChanceToAvoidColdDamageEssence7`*<br>(9-10)% chance to Avoid Cold Damage from Hits |
| **Shield** | `ChanceToAvoidColdDamageEssence5`*<br>(7-8)% chance to Avoid Cold Damage from Hits | `ChanceToAvoidColdDamageEssence6`*<br>(8-9)% chance to Avoid Cold Damage from Hits | `ChanceToAvoidColdDamageEssence7`*<br>(9-10)% chance to Avoid Cold Damage from Hits |

### Essence of Wrath

Mod Group per slot: Boots → `LightningResistance` (S); Gloves → `LightningResistance` (S); Helmet → `LightningResistance` (S); Body Armour → `LightningResistance` (S); Shield → `LightningResistance` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `LightningResist6`<br>+(36-41)% to Lightning Resistance | `LightningResist7`<br>+(42-45)% to Lightning Resistance | `LightningResist8`<br>+(46-48)% to Lightning Resistance |
| **Gloves** | `LightningResist6`<br>+(36-41)% to Lightning Resistance | `LightningResist7`<br>+(42-45)% to Lightning Resistance | `LightningResist8`<br>+(46-48)% to Lightning Resistance |
| **Helmet** | `LightningResist6`<br>+(36-41)% to Lightning Resistance | `LightningResist7`<br>+(42-45)% to Lightning Resistance | `LightningResist8`<br>+(46-48)% to Lightning Resistance |
| **Body Armour** | `LightningResist6`<br>+(36-41)% to Lightning Resistance | `LightningResist7`<br>+(42-45)% to Lightning Resistance | `LightningResist8`<br>+(46-48)% to Lightning Resistance |
| **Shield** | `LightningResist6`<br>+(36-41)% to Lightning Resistance | `LightningResist7`<br>+(42-45)% to Lightning Resistance | `LightningResist8`<br>+(46-48)% to Lightning Resistance |

### Essence of Anguish

Mod Group per slot: Boots → `AvoidIgnite` (S); Gloves → `FireDamage` (P); Helmet → `AvoidIgnite` (S); Body Armour → `FireDamageAvoidance` (S); Shield → `FireDamageAvoidance` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `ChanceToAvoidIgniteEssence5`*<br>(47-50)% chance to Avoid being Ignited | `ChanceToAvoidIgniteEssence6`*<br>(51-55)% chance to Avoid being Ignited | `ChanceToAvoidIgniteEssence7_`*<br>(56-60)% chance to Avoid being Ignited |
| **Gloves** | `AddedFireDamage4`<br>Adds (7-10) to (15-18) Fire Damage to Attacks | `AddedFireDamage5`<br>Adds (9-12) to (19-22) Fire Damage to Attacks | `AddedFireDamage6`<br>Adds (11-15) to (23-27) Fire Damage to Attacks |
| **Helmet** | `ChanceToAvoidIgniteEssence5`*<br>(47-50)% chance to Avoid being Ignited | `ChanceToAvoidIgniteEssence6`*<br>(51-55)% chance to Avoid being Ignited | `ChanceToAvoidIgniteEssence7_`*<br>(56-60)% chance to Avoid being Ignited |
| **Body Armour** | `ChanceToAvoidFireDamageEssence5`*<br>(7-8)% chance to Avoid Fire Damage from Hits | `ChanceToAvoidFireDamageEssence6`*<br>(8-9)% chance to Avoid Fire Damage from Hits | `ChanceToAvoidFireDamageEssence7`*<br>(9-10)% chance to Avoid Fire Damage from Hits |
| **Shield** | `ChanceToAvoidFireDamageEssence5`*<br>(7-8)% chance to Avoid Fire Damage from Hits | `ChanceToAvoidFireDamageEssence6`*<br>(8-9)% chance to Avoid Fire Damage from Hits | `ChanceToAvoidFireDamageEssence7`*<br>(9-10)% chance to Avoid Fire Damage from Hits |

### Essence of Loathing

Mod Group per slot: Boots → `AvoidElementalStatusAilments` (S); Gloves → `CriticalStrikeChanceIncrease` (S); Helmet → `ReducedManaReservationsCost` (S); Body Armour → `ReducedManaReservationsCost` (S); Shield → `IncreasedShieldBlockPercentage` (P).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `ChanceToAvoidElementalStatusAilmentsEssence2`*<br>(21-25)% chance to Avoid Elemental Ailments | `ChanceToAvoidElementalStatusAilmentsEssence3`*<br>(26-30)% chance to Avoid Elemental Ailments | `ChanceToAvoidElementalStatusAilmentsEssence4`*<br>(31-35)% chance to Avoid Elemental Ailments |
| **Gloves** | `CriticalStrikeChanceEssenceGloves5`*<br>(18-20)% increased Global Critical Strike Chance | `CriticalStrikeChanceEssenceGloves6`*<br>(21-23)% increased Global Critical Strike Chance | `CriticalStrikeChanceEssenceGloves7`*<br>(24-26)% increased Global Critical Strike Chance |
| **Helmet** | `ManaReservationEfficiencyEssence5__`*<br>(5-6)% increased Mana Reservation Efficiency of Skills | `ManaReservationEfficiencyEssence6_`*<br>(7-8)% increased Mana Reservation Efficiency of Skills | `ManaReservationEfficiencyEssence7`*<br>(9-10)% increased Mana Reservation Efficiency of Skills |
| **Body Armour** | `ManaReservationEfficiencyEssence5__`*<br>(5-6)% increased Mana Reservation Efficiency of Skills | `ManaReservationEfficiencyEssence6_`*<br>(7-8)% increased Mana Reservation Efficiency of Skills | `ManaReservationEfficiencyEssence7`*<br>(9-10)% increased Mana Reservation Efficiency of Skills |
| **Shield** | `LocalIncreasedBlockPercentage4`<br>(58-63)% increased Chance to Block | `LocalIncreasedBlockPercentage5`<br>(64-69)% increased Chance to Block | `LocalIncreasedBlockPercentage6`<br>(70-75)% increased Chance to Block |

### Essence of Spite

Mod Group per slot: Boots → `Intelligence` (S); Gloves → `Intelligence` (S); Helmet → `Intelligence` (S); Body Armour → `Intelligence` (S); Shield → `Intelligence` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `Intelligence6`<br>+(33-37) to Intelligence | `Intelligence8`<br>+(43-50) to Intelligence | `IntelligenceEssence7`*<br>+(51-58) to Intelligence |
| **Gloves** | `Intelligence6`<br>+(33-37) to Intelligence | `Intelligence8`<br>+(43-50) to Intelligence | `IntelligenceEssence7`*<br>+(51-58) to Intelligence |
| **Helmet** | `Intelligence6`<br>+(33-37) to Intelligence | `Intelligence8`<br>+(43-50) to Intelligence | `IntelligenceEssence7`*<br>+(51-58) to Intelligence |
| **Body Armour** | `Intelligence6`<br>+(33-37) to Intelligence | `Intelligence8`<br>+(43-50) to Intelligence | `IntelligenceEssence7`*<br>+(51-58) to Intelligence |
| **Shield** | `Intelligence6`<br>+(33-37) to Intelligence | `Intelligence8`<br>+(43-50) to Intelligence | `IntelligenceEssence7`*<br>+(51-58) to Intelligence |

### Essence of Zeal

Mod Group per slot: Boots → `MovementVelocity` (P); Gloves → `IncreasedAttackSpeed` (S); Helmet → `WarcrySpeed` (S); Body Armour → `SummonTotemCastSpeed` (S); Shield → `StunRecovery` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `MovementVelocity4`<br>25% increased Movement Speed | `MovementVelocity5`<br>30% increased Movement Speed | `MovementVelocityEssence7`*<br>32% increased Movement Speed |
| **Gloves** | `IncreasedAttackSpeed3`<br>(11-13)% increased Attack Speed | `IncreasedAttackSpeed4`<br>(14-16)% increased Attack Speed | `IncreasedAttackSpeedEssenceGloves7`*<br>(17-18)% increased Attack Speed |
| **Helmet** | `WarcrySpeedEssence2`*<br>(21-25)% increased Warcry Speed | `WarcrySpeedEssence3`*<br>(26-30)% increased Warcry Speed | `WarcrySpeedEssence4_`*<br>(31-35)% increased Warcry Speed |
| **Body Armour** | `SummonTotemCastSpeedEssence2`*<br>(26-30)% increased Totem Placement speed | `SummonTotemCastSpeedEssence3`*<br>(31-35)% increased Totem Placement speed | `SummonTotemCastSpeedEssence4`*<br>(36-45)% increased Totem Placement speed |
| **Shield** | `StunRecovery5`<br>(23-25)% increased Stun and Block Recovery | `StunRecovery6`<br>(26-28)% increased Stun and Block Recovery | `StunRecoveryEssence7`*<br>(29-34)% increased Stun and Block Recovery |

### Essence of Dread

Mod Group per slot: Boots → `BaseLocalDefences` (P); Gloves → `BaseLocalDefences` (P); Helmet → `BaseLocalDefences` (P); Body Armour → `BaseLocalDefences` (P); Shield → `BaseLocalDefences` (P).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `LocalIncreasedPhysicalDamageReductionRatingEssenceBootsGloves5`*<br>+(91-105) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceBootsGloves6`*<br>+(106-120) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceBootsGloves7`*<br>+(121-135) to Armour |
| **Gloves** | `LocalIncreasedPhysicalDamageReductionRatingEssenceBootsGloves5`*<br>+(91-105) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceBootsGloves6`*<br>+(106-120) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceBootsGloves7`*<br>+(121-135) to Armour |
| **Helmet** | `LocalIncreasedPhysicalDamageReductionRatingEssenceHelm5`*<br>+(121-140) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceHelm6_`*<br>+(141-160) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceHelm7_`*<br>+(161-180) to Armour |
| **Body Armour** | `LocalIncreasedPhysicalDamageReductionRating8`<br>+(151-200) to Armour | `LocalIncreasedPhysicalDamageReductionRating10`<br>+(301-400) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssence7`*<br>+(390-475) to Armour |
| **Shield** | `LocalIncreasedPhysicalDamageReductionRatingEssenceShield5_`*<br>+(151-225) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceShield6`*<br>+(226-300) to Armour | `LocalIncreasedPhysicalDamageReductionRatingEssenceShield7____`*<br>+(301-375) to Armour |

### Essence of Envy

Mod Group per slot: Boots → `ChaosResistance` (S); Gloves → `ChaosResistance` (S); Helmet → `ChaosResistance` (S); Body Armour → `ChaosResistance` (S); Shield → `ChaosResistance` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `ChaosResist4`<br>+(21-25)% to Chaos Resistance | `ChaosResist5`<br>+(26-30)% to Chaos Resistance | `ChaosResist6`<br>+(31-35)% to Chaos Resistance |
| **Gloves** | `ChaosResist4`<br>+(21-25)% to Chaos Resistance | `ChaosResist5`<br>+(26-30)% to Chaos Resistance | `ChaosResist6`<br>+(31-35)% to Chaos Resistance |
| **Helmet** | `ChaosResist4`<br>+(21-25)% to Chaos Resistance | `ChaosResist5`<br>+(26-30)% to Chaos Resistance | `ChaosResist6`<br>+(31-35)% to Chaos Resistance |
| **Body Armour** | `ChaosResist4`<br>+(21-25)% to Chaos Resistance | `ChaosResist5`<br>+(26-30)% to Chaos Resistance | `ChaosResist6`<br>+(31-35)% to Chaos Resistance |
| **Shield** | `ChaosResist4`<br>+(21-25)% to Chaos Resistance | `ChaosResist5`<br>+(26-30)% to Chaos Resistance | `ChaosResist6`<br>+(31-35)% to Chaos Resistance |

### Essence of Misery

Mod Group per slot: Boots → `IncreasedMana` (P); Gloves → `IncreasedMana` (P); Helmet → `IncreasedMana` (P); Body Armour → `IncreasedMana` (P); Shield → `ManaRegeneration` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `IncreasedMana9`<br>+(55-59) to maximum Mana | `IncreasedMana11`<br>+(65-68) to maximum Mana | `IncreasedManaEssence7`*<br>+(69-77) to maximum Mana |
| **Gloves** | `IncreasedMana9`<br>+(55-59) to maximum Mana | `IncreasedMana11`<br>+(65-68) to maximum Mana | `IncreasedManaEssence7`*<br>+(69-77) to maximum Mana |
| **Helmet** | `IncreasedMana9`<br>+(55-59) to maximum Mana | `IncreasedMana11`<br>+(65-68) to maximum Mana | `IncreasedManaEssence7`*<br>+(69-77) to maximum Mana |
| **Body Armour** | `IncreasedMana9`<br>+(55-59) to maximum Mana | `IncreasedMana11`<br>+(65-68) to maximum Mana | `IncreasedManaEssence7`*<br>+(69-77) to maximum Mana |
| **Shield** | `ManaRegeneration5`<br>(50-59)% increased Mana Regeneration Rate | `ManaRegeneration6`<br>(60-69)% increased Mana Regeneration Rate | `ManaRegenerationEssence7_`*<br>(70-76)% increased Mana Regeneration Rate |

### Essence of Scorn

Mod Group per slot: Boots → `AvoidStun` (S); Gloves → `AvoidStun` (S); Helmet → `AvoidStun` (S); Body Armour → `IncreasedStunThreshold` (S); Shield → `SpellCriticalStrikeChanceIncrease` (S).

| slot | Screaming (T5) | Shrieking (T6) | Deafening (T7) |
| --- | --- | --- | --- |
| **Boots** | `StunAvoidanceEssence5`*<br>(23-26)% chance to Avoid being Stunned | `StunAvoidanceEssence6`*<br>(27-30)% chance to Avoid being Stunned | `StunAvoidanceEssence7`*<br>(31-44)% chance to Avoid being Stunned |
| **Gloves** | `StunAvoidanceEssence5`*<br>(23-26)% chance to Avoid being Stunned | `StunAvoidanceEssence6`*<br>(27-30)% chance to Avoid being Stunned | `StunAvoidanceEssence7`*<br>(31-44)% chance to Avoid being Stunned |
| **Helmet** | `StunAvoidanceEssence5`*<br>(23-26)% chance to Avoid being Stunned | `StunAvoidanceEssence6`*<br>(27-30)% chance to Avoid being Stunned | `StunAvoidanceEssence7`*<br>(31-44)% chance to Avoid being Stunned |
| **Body Armour** | `IncreasedStunThresholdEssence5`*<br>(31-39)% increased Stun Threshold | `IncreasedStunThresholdEssence6`*<br>(40-45)% increased Stun Threshold | `IncreasedStunThresholdEssence7`*<br>(46-60)% increased Stun Threshold |
| **Shield** | `SpellCriticalStrikeChance5`<br>(80-89)% increased Spell Critical Strike Chance | `SpellCriticalStrikeChance6_`<br>(90-109)% increased Spell Critical Strike Chance | `SpellCriticalStrikeChanceEssence7`*<br>(110-119)% increased Spell Critical Strike Chance |

### The tier-8 essences

| slot | Essence of Delirium | Essence of Desolation | Essence of Horror | Essence of Hysteria | Essence of Insanity |
| --- | --- | --- | --- | --- | --- |
| **Boots** | `CannotBePoisonedEssence1`*<br>Cannot be Poisoned | `MovementSpeedPerNearbyEnemyEssence1`*<br>5% increased Movement Speed for each nearby Enemy, up to a maximum of 50% | `ElementalDamageTakenWhileStationaryEssence1`*<br>5% reduced Elemental Damage Taken while stationary | `BurningGroundWhileMovingEssence1`*<br>Drops Burning Ground while moving, dealing 2500 Fire Damage per second for 4 seconds | `ManaRegenerationWhileShockedEssence1`*<br>70% increased Mana Regeneration Rate while Shocked |
| **Gloves** | `SupportDamageOverTimeEssence1`*<br>Socketed Gems deal 30% more Damage over Time | `AttackCastSpeedPerNearbyEnemyEssence1`*<br>5% increased Attack and Cast Speed for each nearby Enemy, up to a maximum of 30% | `SocketedSkillsCriticalChanceEssence1`*<br>Socketed Gems have +3.5% Critical Strike Chance | `SocketedGemsDealAdditionalFireDamageEssence1`*<br>Socketed Gems deal 175 to 225 Added Fire Damage | `SocketedGemsHaveMoreAttackAndCastSpeedEssenceNew1`*<br>Socketed Gems have 16% more Attack and Cast Speed |
| **Helmet** | `SocketedAuraGemLevelsEssence1`*<br>+2 to Level of Socketed Aura Gems | `LocalGemLevelIfOnlySocketedGemEssence1`*<br>+6 to Level of Socketed Gems while there is a single Gem Socketed in this Item | `SocketedGemsDealMoreElementalDamageEssence1`*<br>Socketed Gems deal 30% more Elemental Damage | `SocketedSkillDamageOnLowLifeEssence1__`*<br>Socketed Gems deal 30% more Damage while on Low Life | `SocketedGemsAddPercentageOfPhysicalAsLightningEssence1`*<br>Socketed Gems gain 50% of Physical Damage as extra Lightning Damage |
| **Body Armour** | `ChaosDamageOverTimeTakenEssence1`*<br>25% reduced Chaos Damage taken over time | `GlobalDefencesNoOtherDefenceModifiersOnEquipmentEssence1`*<br>(70-90)% increased Global Defences if there are no Defence Modifiers on other Equipped Items | `ReducedDamageFromCriticalStrikesPerEnduranceChargeEssence1`*<br>You take 10% reduced Extra Damage from Critical Strikes per Endurance Charge | `AreaOfEffectEssence1`*<br>25% increased Area of Effect | `OnslaughtWhenHitNewEssence1`*<br>You gain Onslaught for 6 seconds when Hit |
| **Shield** | `SpellBlockOnLowLifeEssence1`*<br>+15% Chance to Block Spell Damage while on Low Life | `ArmourAppliesElementalHitsIfBlockedRecentlyEssence1`*<br>(2-4)% of Armour applies to Fire, Cold and Lightning Damage taken from Hits if you have Blocked Recently | `NearbyEnemiesChilledOnBlockEssence1`*<br>Chill Nearby Enemies when you Block | `AddedFireDamageIfBlockedRecentlyEssence1`*<br>Adds 60 to 100 Fire Damage if you've Blocked Recently | `PowerChargeOnBlockEssence1`*<br>25% chance to gain a Power Charge when you Block |

The four corruption-only essences (Delirium, Horror, Hysteria, Insanity) grant a **suffix on all
five slots**; Desolation grants a **prefix on four of five** (a suffix on gloves). None of their
ranges overlaps a normal tier, because their rendered lines appear nowhere else in the pool — with
one wrinkle worth naming: Desolation's boots prefix
`MovementSpeedPerNearbyEnemyEssence1` sits in group **`MovementVelocity`**, the same group as the
normal movement-speed ladder, while rendering a completely different line. So one Mod Group can hold
two unrelated rendered lines, which is the case discussed under
[Why essence mods must not join the tier ladder](#why-essence-mods-must-not-join-the-tier-ladder).

---

## The acceptance craft: boots with an essence-granted movement speed mod

Four of the 65 in-scope essences put a mod from the `MovementVelocity` group on boots. Three of
them are the Zeal ladder, and **only the Deafening one is an essence-only mod**; the fourth is
Essence of Desolation, whose mod is in the same group but is a different line entirely:

| essence | mod id | affix name | prefix/suffix | required ilvl | value | in the normal pool? |
| --- | --- | --- | --- | --- | --- | --- |
| Screaming Essence of Zeal (`CurrencyEssenceZeal2`) | `MovementVelocity4` | `Gazelle's` | prefix | 40 | `25% increased Movement Speed` | **yes** — normal T3 on boots |
| Shrieking Essence of Zeal (`CurrencyEssenceZeal3`) | `MovementVelocity5` | `Cheetah's` | prefix | 55 | `30% increased Movement Speed` | **yes** — normal T2 on boots |
| Deafening Essence of Zeal (`CurrencyEssenceZeal4`) | `MovementVelocityEssence7` | `Essences` | prefix | 82 | `32% increased Movement Speed` | **no** — `is_essence_only: true` |
| Essence of Desolation (`CurrencyEssenceFaridun1`) | `MovementSpeedPerNearbyEnemyEssence1` | `Essences` | prefix | 63 | `5% increased Movement Speed for each nearby Enemy, up to a maximum of 50%` | **no** — `is_essence_only: true`, and a **different rendered line** in the same Mod Group |

Group is `MovementVelocity` for all four. The normal ladder on boots, as
`mods_by_base.json` resolves it for e.g. `dex_armour,boots,armour,default`
(*Rawhide Boots*), with GGG's ilvl-descending tier numbering:

```
T1  MovementVelocity6         ilvl 86   w1000   35% increased Movement Speed
T2  MovementVelocity5         ilvl 55   w1000   30% increased Movement Speed     ← Shrieking Zeal
T3  MovementVelocity4         ilvl 40   w1000   25% increased Movement Speed     ← Screaming Zeal
T4  MovementVelocity3         ilvl 30   w1000   20% increased Movement Speed     ← Wailing Zeal (out of scope)
T5  MovementVelocity2         ilvl 15   w1000   15% increased Movement Speed
T6  MovementVelocity2Royale   ilvl  5   w1000   (15-25)% increased Movement Speed   ← see the Royale note
T7  MovementVelocity1         ilvl  1   w1000   10% increased Movement Speed

essence-only, no spawn weight anywhere:
    MovementVelocityEssence7            ilvl 82           32% increased Movement Speed     ← Deafening Zeal
    MovementSpeedPerNearbyEnemyEssence1 ilvl 63           5% … for each nearby Enemy …     ← Desolation
```

So for the acceptance craft the app must recognise **`32% increased Movement Speed`** as a
legitimate mod on boots. It is not in the normal pool, its numeric value falls between normal T2
(30%) and T1 (35%) without touching either, and the clipboard line it produces is:

```
{ Prefix Modifier "Essences" — Speed }
32% increased Movement Speed
```

— **no `(Tier: N)`**. Compare Shrieking Essence of Zeal, whose guaranteed mod *is* a normal mod
and therefore *does* carry one:

```
{ Prefix Modifier "Cheetah's" (Tier: 2) — Speed }
30% increased Movement Speed
```

Both of those must parse, both must satisfy a `MovementVelocity` Member, and only the second one
can be cross-checked against the game's annotation.

### The Royale contamination, which lands squarely on this Mod Group

`MovementVelocity2Royale` (affix name `Sprinter's`, `required_level: 5`,
`(15-25)% increased Movement Speed`) is present in `mods_by_base.json`'s **boots prefix pool with
weight 1000**. It is a Path of Exile: Royale mod and, to the best of my knowledge, cannot appear on
a boots item in the normal game — a crafted 18% movement speed roll does not exist. RePoE carries
**no flag** distinguishing it: `domain: item`, `generation_type: prefix`, `type: MovementVelocity`,
ordinary spawn weights. **The only signal is the `Royale` substring in the mod id.**

I could not verify this against poedb — the base pages I fetched
(`poedb.tw/us/Titan_Greaves`, `poedb.tw/us/Stealth_Boots`) do not inline the `new ModsView({…})`
payload that [`mod-tier-data.md`](mod-tier-data.md) lifted from the Ghastly Eye Jewel page, so the
technique the repo already trusts does not reach these bases. **Recorded as unverified.**

Why it matters anyway, because it is decidable from the data:

- It makes the boots `MovementVelocity` ladder **7 tiers deep instead of 6**, so `MovementVelocity1`
  becomes T7 where the game prints `(Tier: 6)` — an annotation disagreement, which
  [#21](https://github.com/Furizaa/poe-graft/issues/21) says `Halt`s.
- Its range `(15-25)%` **overlaps** `MovementVelocity2` (15%), `MovementVelocity3` (20%) and
  `MovementVelocity4` (25%), so `build-mod-pool.py`'s self-check 3 ("no overlapping tier ranges")
  **fails on boots** if it is included.

Exhaustive scan of `mods_by_base.json` for `Royale` mod ids in the five in-scope classes:

| class | Royale mod ids in the prefix/suffix pools |
| --- | --- |
| Boots | `MovementVelocity2Royale` |
| Gloves | `IncreasedAttackSpeed2Royale`, `IncreasedCastSpeed2Royale` |
| Shields | `IncreasedAttackSpeed2Royale`, `IncreasedCastSpeed2Royale` |
| Helmets | — none |
| Body Armours | — none |

(Also present on every weapon class as `LocalIncreasedAttackSpeed2Royale____`, and on Amulets and
Quivers — noted for the "other item classes" fog on #21.)

This is a **pre-existing** hazard, not one essences introduce. It just happens to sit on the
acceptance craft's Mod Group, so the pipeline ticket will meet it on day one.

---

## How essence mods relate to the normal pool's tiers of the same Mod Group

Three separate relationships, and they need separating because they have different consequences.

### 1. Some essences guarantee an ordinary pool mod

53 of the 171 distinct guaranteed mods on the five armour slots are ordinary pool mods with real
spawn weights. Resistances are the clearest case: Essence of Hatred hands out
`ColdResist6/7/8` — normal T3/T2/T1 cold resistance — at Screaming/Shrieking/Deafening. So
`Deafening Essence of Hatred` on boots is *exactly* the normal T1 suffix `of Haast`,
`+(46-48)% to Cold Resistance`, complete with `(Tier: 1)` in the clipboard. Nothing new to
recognise; the essence simply forces a draw the pool already contains.

The same is true for Anger (fire res), Wrath (lightning res), Envy (chaos res), Rage (Strength),
Sorrow (Dexterity), Spite (Intelligence) at Screaming/Shrieking, and Misery (Mana) at
Screaming/Shrieking. Deafening usually — not always — steps off the ladder onto an essence-only mod.

### 2. Others are essence-only mods **inside** an existing Mod Group

This is the common case and the awkward one. `MovementVelocityEssence7` is in group
`MovementVelocity`. `IncreasedLifeEssence5/6` and `IncreasedLifeEssenceBootsGloves1` are in
`IncreasedLife`. `DexterityEssence7` is in `Dexterity`. They are group-mates of normal tiers,
which means:

- the **one-mod-per-group** exclusion applies as usual: an essence-granted
  `+(91-105) to maximum Life` occupies the `IncreasedLife` slot, so no normal life tier can also
  roll. Good for the odds model, and it means a Member on that group is satisfied by either.
- their **ranges overlap normal tiers**, because the essence ladder was balanced independently of
  the normal ladder. Measured per Base Group (the unit #21 chose for a pool file), by resolving each
  distinct base tag-set's pool from `mods.json` spawn weights and comparing every essence-only mod
  against same-group same-stat normal mods:

| class | Base Groups (distinct base tag-sets) | with ≥1 overlapping essence/normal pair |
| --- | --- | --- |
| Boots | 28 | **28** |
| Gloves | 27 | **27** |
| Helmet | 24 | **24** |
| Body Armour | 21 | **21** |
| Shield | 18 | **17** |

Worked example, `dex_armour,boots,armour,default` (7 bases, e.g. *Rawhide Boots*):

```
IncreasedLifeEssence5              +(61-75) to maximum Life   overlaps IncreasedLife4  ilvl24  +(55-69)
                                                              overlaps IncreasedLife5  ilvl30  +(70-84)
IncreasedLifeEssence6              +(76-90) to maximum Life   overlaps IncreasedLife5  ilvl30  +(70-84)
                                                              overlaps IncreasedLife6  ilvl36  +(85-99)
IncreasedLifeEssenceBootsGloves1   +(91-105) to maximum Life  overlaps IncreasedLife6  ilvl36  +(85-99)
                                                              overlaps IncreasedLife7  ilvl44  +(100-114)
DexterityEssence7                  +(51-58) to Dexterity      overlaps Dexterity9      ilvl82  +(51-55)
LocalIncreasedEvasionRatingEssenceGlovesBoots5  +(91-105) Evasion  overlaps LocalIncreasedEvasionRating5  +(83-101)
                                                                   overlaps LocalIncreasedEvasionRating6  +(102-120)
```

A roll of `+65 to maximum Life` on essence-crafted boots is therefore genuinely ambiguous by
numbers alone: `IncreasedLife4`, `IncreasedLife5` and `IncreasedLifeEssence5` all admit it.
**The affix name resolves it** — `Fecund`/`Virile`/… for the normal tiers versus `Essences` for the
essence mod — so the Alt+Ctrl+C path is not merely the preferred path here, it is the *only* one
that disambiguates. The plain-Ctrl+C fallback described in
[`mod-tier-data.md`](mod-tier-data.md#fallback-path-plain-ctrlc-no-advanced-descriptions) cannot
tell these apart, which is a second, independent reason
[#21](https://github.com/Furizaa/poe-graft/issues/21)'s "Advanced Mod Descriptions stops being
optional" is correct.

### 3. And some are the only member of their group

`WarcrySpeedEssence2/3/4_`, `SummonTotemCastSpeedEssence2/3/4`,
`ChanceToAvoidShockEssence5/6/7`, `StunAvoidanceEssence5/6/7`,
`ChanceToAvoid{Fire,Cold,Lightning}DamageEssence5/6/7`, and every tier-8 mod — these sit in groups
with no normal-pool member on the slot at all. They are unambiguous by construction: their
rendered line appears nowhere else, and no overlap is possible.

### There is no essence "tier" in the export

RePoE gives an essence-only mod **no tier number**. The three orderings that exist are:

1. the **essence's own `level`** (5 → 6 → 7, and 8 for the specials), which is the ladder a player
   thinks in;
2. the mod's **`required_level`** (58 → 74 → 82; 63 for tier 8), which mirrors (1) exactly for the
   in-scope tiers bar the three `ChanceToAvoidFreezeEssence*` outliers at ilvl 1;
3. nothing else.

So if a Member needs a Tier Threshold over an essence ladder, the pipeline has to *mint* the
numbering — from the essence level, which is the only meaningful one. **The game will not confirm
it**, because it prints no annotation for these mods.

### Why essence mods must not join the tier ladder

Concretely, on boots: the normal `MovementVelocity` ladder has `required_level`s
`86, 55, 40, 30, 15, (5), 1`. If `MovementVelocityEssence7` (ilvl 82) were appended to the group and
the group re-sorted ilvl-descending the way `build-mod-pool.py` numbers tiers, you would get
`86, 82, 55, 40, …` — and `MovementVelocity5` would be derived as **T3** while the game prints
**`(Tier: 2)`**. Every `Cheetah's` roll would then trip the annotation-disagrees Diagnostic, which
[#21](https://github.com/Furizaa/poe-graft/issues/21) says `Halt`s. The runtime `(Tier: N)`
cross-check — the thing standing in for a poedb scraper on this map — would be broken by our own
data.

Two further reasons the same way:

- `build-mod-pool.py` line 166 asserts every tier in a group renders the **same** `match_string`
  list. `MovementVelocity` would then hold both `#% increased Movement Speed` and
  `#% increased Movement Speed for each nearby Enemy, up to a maximum of #%`, and the assertion
  fires. (Note this assertion is **already** unsatisfiable on armour independently of essences:
  flat Armour, flat Evasion and flat Energy Shield are all group `BaseLocalDefences`, three
  different rendered lines in one group. The pipeline ticket has to relax it either way.)
- self-check 3, "no overlapping tier ranges", fails in every armour Base Group as measured above.

**Recommendation:** keep the essence mods in a **sibling list** — e.g. an `essence_pool` array
alongside `prefixes` / `suffixes`, each row carrying its `group`, `generation_type`,
`match_lines`, `stats`, `essence_id` and `essence_level` — and let the matcher consult it as a
second index. That preserves the three properties this project's trust story rests on: the derived
tier numbers keep agreeing with the game's annotation, `$schema_id: poe-graft/base-mod-pool@1`'s
`prefixes`/`suffixes` keep the shape `pool.rs` reads today, and a pool file stays readable against
poedb by eye. It also matches the shape `non_alteration_pools` already uses for corrupted and delve
mods — mods that must be *recognised* but can never come from the currency being spammed.

---

## Open questions

Recorded as unknowns rather than smoothed over. The first two are the ones that could bite.

- **Does the guaranteed mod ignore the item's item level?** Not established. The essence mods for
  Deafening sit at `required_level: 82`; if the item's level gated them, a Deafening Essence of Zeal
  on ilvl-75 boots could not produce `32% increased Movement Speed`, which would contradict
  "guaranteeing one property". Community guides say an essence "can guarantee [a stat] even though
  the item would normally be unable to roll [it]"
  ([maxroll](https://maxroll.gg/poe/leagues/essence-league-guide),
  [goldkk](https://www.goldkk.com/news/2144--path-of-exile-essences-guide--how-to-get-essences--how-to-use-them-effectively)),
  but that sentence is about *spawn weights*, not item level, and neither is a primary source.
  **I would not encode either answer.** This matters because #21 decided that provably impossible
  rules are refused at the first Read using `required_ilvl` — and if the guaranteed Member's mod is
  ilvl-exempt, that check would wrongly refuse a legitimate craft. The cheap resolution is one
  observation on the gaming PC: one Deafening essence on a sub-82 base settles it forever.
- **How many affixes does an essence-applied Rare end up with?** Not established. Craft of Exile
  describes essences as "essentially alchemy orbs that always grant one guaranteed mod"
  ([craftofexile.com/basics](https://www.craftofexile.com/basics)), which implies the Rare affix-count
  distribution, but I found no datamined table and did not verify the rare distribution the way
  [`mod-tier-data.md`](mod-tier-data.md) verified the magic 50/50 against two simulators.
  `data/ghastly-eye-jewel.json` currently carries only `rare_jewel` odds, which are jewel-specific.
  **This affects only the odds figure**, which #21 already declared advisory and non-gating, so it is
  safe to ship the parsing work without it — but the odds simulator ticket needs its own answer.
- **Does the game really print no `(Tier: N)` for essence mods?** I am confident, from four
  independent real Alt+Ctrl+C captures in unrelated projects, all showing
  `{ Prefix Modifier "Essences" — … }` with no tier while neighbouring normal mods carry one:
  [PathOfBuilding `spec/System/TestItemParse_spec.lua`](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/dev/spec/System/TestItemParse_spec.lua),
  [xiletrade `weapon-rare.txt`](https://github.com/maxensas/xiletrade/blob/master/src/Xiletrade.Test/ItemInfoDescription/English/weapon-rare.txt),
  [poe-item-parser `examples/dagger.txt`](https://github.com/gailingmic/poe-item-parser/blob/main/examples/dagger.txt),
  [poe-recomb `proto/ctrlaltc.txt`](https://github.com/OrderedSet86/poe-recomb/blob/main/proto/ctrlaltc.txt).
  One project's *synthetic* unit-test string does contain `{ Prefix Modifier "Essences" (Tier: 1) — Life }`
  ([poenavi](https://github.com/buri34/poenavi)), which is why I am calling this confident rather
  than certain: it is a hand-written fixture, not a capture, but it means at least one author
  believed a tier can appear. **The app should treat the annotation as optional here and never
  require it** — which is what #21's "if the annotation is absent it fails closed to Hit" already
  does. A *disagreeing* annotation still `Halt`s, and that is the case to watch.
- **Is `MovementVelocity2Royale` (and the gloves/shield equivalents) really absent from the live
  game?** Unverified — see the Royale note above. The pipeline ticket should filter mod ids
  containing `Royale` and say so in a comment, but the filter is a judgement call, not a datamined
  fact.
- **Is `Essence of Desolation` obtainable in the current league?** Its id is
  `CurrencyEssenceFaridun1` and community sources tie it to the Mirage league
  ([poe.ninja](https://poe.ninja/poe1/economy/mirage/essences/essence-of-desolation)). Whether it
  still drops in 3.29 I did not establish. It costs nothing to include: it is one more row in the
  picker and five more mods in the pool.
- **125 `is_essence_only` mods are referenced by no essence in `essences.json`** — e.g.
  `AdditionalShieldBlockChance1–4`, `ChanceToDodgeEssence4–7`,
  `AddedColdDamageEssenceQuiverGloves4–7`. These are legacy or superseded essence mods. Because an
  essence-only mod has no spawn weights, **there is no way in this export to attribute them to an
  item class** — the `essences.json` map is the only attribution, and they are absent from it. So a
  legacy essence-crafted item, or one whose essence mod was since rebalanced, could present a line
  the pool data cannot recognise, and the app would `Halt`. That is the correct behaviour under
  #21's rule; it is recorded here so it is not discovered mid-craft. It cannot be fixed from the
  export.
- **Lower tiers were not tabulated.** Whispering through Wailing are out of scope, so I did not
  build their mod table. If they are ever wanted (they can create a Rare from a Normal, which is a
  different craft), the join is identical — just drop the `level >= 5` filter.
- **Nothing here is verified against a second datamine.** #4 cross-read RePoE against poedb's
  inlined `ModsView` payload for the jewel. The equivalent payload is not present on poedb's armour
  base pages, and poedb has no per-essence mod-id listing I could machine-read, so the
  essence → mod id mapping rests on RePoE alone. What I *did* cross-read against poedb is the part
  that is not in RePoE at all: the per-tier item description text, which is the answer to the
  ticket's first question.
