# Ghastly Eye Jewel mod and tier data — where it comes from and what shape it takes

Research for [#4](https://github.com/Furizaa/poe-graft/issues/4). Captured 2026-08-04 against
game data version **3.29.1.2.2** (the live PoE 1 patch, per
[poe-tool-dev/latest-patch-version](https://github.com/poe-tool-dev/latest-patch-version)).

Artefacts produced alongside this document:

- [`data/ghastly-eye-jewel.json`](../../data/ghastly-eye-jewel.json) — the full mod pool, 66
  prefixes and 60 suffixes across 66 groups, with tiers, ranges, ilvl gates and spawn weights.
- [`data/raw/`](../../data/raw) — the verbatim upstream slices this was derived from, so the
  derivation is re-checkable without re-downloading 75 MB.
- [`scripts/build-mod-pool.py`](../../scripts/build-mod-pool.py) — the generator, with its
  self-checks. See [Reproducing this](#reproducing-this).

---

## Verdict

**Source: RePoE, the `repoe-fork` export.** `https://repoe-fork.github.io/` (repo:
[repoe-fork/repoe](https://github.com/repoe-fork/repoe)). It is GGG-datamined JSON, generated
by PyPoE from the GGPK using the poe-tool-dev `dat-schema`, is current with the live patch, and
is the upstream that every other tool in this space consumes. The original `brather1ng/RePoE`
is dead (last push 2022-09-06) — do not use it.

**Shape: a committed, generated JSON file per base, keyed by mod group, with tiers as an array.**
Ship it in the repo; do not fetch at runtime. Reasons: the app must work with the game running
and no assumptions about network, the data changes only on a GGG patch (a handful of times a
league), and a runtime fetch turns a 200-line matcher into a cache-invalidation problem. Refresh
it with a checked-in generator script and a version stamp; see [Staleness](#staleness).

**The single most important finding, which changes the shape of ticket "The mod matcher":**
with *Advanced Mod Descriptions* enabled and copying with **Alt+Ctrl+C** instead of Ctrl+C, the
game hands you the mod's affix name **and its tier number** in the clipboard text, plus the
roll's own min–max bounds inline. Reverse-matching rendered text to a mod id is a *fallback*,
not the primary mechanism. See
[Rendered text → mod → tier](#rendered-text--mod-identity--tier).

### Recommended data model

Per **base** (`data/<base-slug>.json`):

| field | why |
| --- | --- |
| `base.base_id` / `name` / `item_class` / `domain` / `tags` | identity; `tags` is the key into the mod pool and is how you'd add a second base later |
| `base.affix_slots` | magic = 1 prefix + 1 suffix; rare jewel = 2 + 2. Bounds the model |
| `affix_count_odds` | how many affixes a reforge produces (magic 1:1); needed only for the probability display |
| `matching` | the two regexes the parser needs, stored next to the data so they cannot drift from it |
| `pool_totals_by_ilvl` | precomputed prefix/suffix weight totals at every ilvl breakpoint, so probability is a lookup |
| `prefixes[]`, `suffixes[]` | one entry per **mod group** |
| `non_alteration_pools` | corrupted / delve mods, so the matcher can recognise a line it must *not* treat as a hit |

Per **mod group** (this is the unit the UI picks, and the unit of mutual exclusion — GGG allows
only one mod per group on an item):

```json
{
  "group": "MinionAddedPhysicalDamage",
  "generation_type": "prefix",
  "match_lines": [
    { "match_string": "Minions deal # to # additional Physical Damage",
      "stat_ids": ["minion_global_minimum_added_physical_damage",
                   "minion_global_maximum_added_physical_damage"],
      "trade_stat_id": "explicit.stat_1172029298",
      "index_handlers": [] }
  ],
  "match_string": "Minions deal # to # additional Physical Damage",
  "tier_count": 6,
  "tiers": [ /* tier 1 first */ ]
}
```

Per **tier**:

```json
{
  "tier": 1,
  "mod_id": "AbyssMinionAddedPhysicalDamageJewel6",
  "affix_name": "Flaring",
  "required_ilvl": 83,
  "spawn_weight": 175,
  "stats": [
    { "id": "minion_global_minimum_added_physical_damage",
      "min": 23, "max": 26, "display_min": 23, "display_max": 26 },
    { "id": "minion_global_maximum_added_physical_damage",
      "min": 33, "max": 39, "display_min": 33, "display_max": 39 }
  ],
  "text": "Minions deal (23-26) to (33-39) additional Physical Damage"
}
```

Design notes that matter:

- **`tier` is 1-based, best first**, and is computed the way GGG computes it: sort the group's
  mods that can spawn on *this base* by `required_ilvl` descending. It is **not** gated by the
  item's own item level, so it matches the `(Tier: N)` the game prints. Verified against the
  poedb tier column.
- **`match_lines` is a list, not a string.** One mod can render as two lines
  (`MinionAttackAndCastSpeed` prints an attack-speed line *and* a cast-speed line), and some
  mods carry a stat with a 0–0 range that renders nothing at all
  (`ChanceToAvoidFreezeAndChill` carries a dead `base_avoid_chill_%`). A single `match_string`
  cannot express either.
- **`display_min`/`display_max` exist because `min`/`max` are not display units.** RePoE stores
  raw stat values; the tooltip applies `index_handlers`. Nine mods on this base are affected —
  `per_minute_to_per_second` (regen mods: raw 65 renders as `1.1`),
  `divide_by_one_hundred` (`MinionLifeLeech`: raw 30 renders as `0.3`) and `negate` (the four
  "reduced" enchantment suffixes, stored as negative). Compare a parsed roll against the
  `display_*` bounds. The generator self-checks these against RePoE's own rendered `text`
  field: 126/126 tiers agree.
- `trade_stat_id` is carried for free and is the join key to `pathofexile.com/api/trade`.
  Not needed for v1, cheap insurance for a "what's this worth" feature.

Roll → tier is **unambiguous for every mod on this base**: no two tiers of any group have
overlapping display ranges (checked programmatically, 0 overlapping pairs). And no two groups
share a `match_string`. So on this base the fallback matcher is exact, not heuristic.

---

## The Tier-1 numbers, verified

| claim in #1/#4 | verdict |
| --- | --- |
| Tier 1 range `23-26` to `33-39` | **correct** |
| It is a **prefix** | **correct** (`generation_type: prefix`) |
| `0.443%` per roll with an Orb of Alteration | **correct as a conditional, misleading as a per-click rate** — see below |

```
mod id          AbyssMinionAddedPhysicalDamageJewel6
affix name      Flaring          (magic item reads "Flaring Ghastly Eye Jewel of …")
group           MinionAddedPhysicalDamage      generation_type  prefix
required ilvl   83               spawn weight  175  (tag: abyss_jewel_summoner)
stats           minion_global_minimum_added_physical_damage  23–26
                minion_global_maximum_added_physical_damage  33–39
rendered        Minions deal (23-26) to (33-39) additional Physical Damage
trade stat id   explicit.stat_1172029298
```

Full tier ladder for the target group (all six tiers, spawn weight in the `w` column):

| tier | mod id | affix | ilvl | w | range |
| --- | --- | --- | --- | --- | --- |
| 1 | `…Jewel6` | Flaring | 83 | 175 | (23-26) to (33-39) |
| 2 | `…Jewel5` | Tempered | 72 | 350 | (18-21) to (24-27) |
| 3 | `…Jewel4` | Razor-sharp | 63 | 700 | (14-17) to (20-23) |
| 4 | `…Jewel3` | Annealed | 54 | 700 | (9-12) to (15-18) |
| 5 | `…Jewel2` | Gleaming | 42 | 700 | (5-8) to (11-14) |
| 6 | `…Jewel1` | Glinting | 1 | 700 | (2-3) to (5-8) |

**A practical constraint for the acceptance test: the jewel must be item level ≥ 83.** Below
that, T1 cannot spawn at all and the app would gate on something unreachable. Worth surfacing
in the UI.

### Probability derivation

Mechanics that bound the model:

1. An **Orb of Alteration** on a magic item removes every explicit and reforges it. It requires
   the item to already be magic (Transmute a normal one first).
2. A **magic item has at most 1 prefix and 1 suffix**, and gets **1 or 2 affixes**. Affix count
   is not in the GGG data files; the two independent open-source simulators that model it agree
   on **1:1 (50/50)** for magic —
   [kalandralang `src/item.ml`](https://github.com/doomeer/kalandralang/blob/master/src/item.ml)
   (`Magic -> let w1 = 50 in let w2 = 50 in …`) and
   [PoeCraftLib `StatFactory.cs`](https://github.com/DanielWieder/PoeCraftLib/blob/master/src/Currency/StatFactory.cs)
   (`MagicAffixCountOdds = { {1,1}, {2,1} }`). This is the weakest link in the chain — it is
   community-reverse-engineered, not datamined. See [Open questions](#open-questions).
3. Affixes are drawn from the **combined** prefix+suffix pool weighted by `spawn_weight`,
   with the pool re-filtered after each draw (slot full, group already used). So a 2-affix
   magic item is always exactly 1 prefix + 1 suffix.
4. A tier can spawn iff **item level ≥ the mod's `required_ilvl`**. Nothing else gates it. So
   the pool — and therefore the denominator — grows with item level.
5. Rolls are independent. The same mod can come up on consecutive alterations; there is no
   memory and no pity.

Weights at **ilvl 86** (the point at which the pool stops growing):

```
w  = 175     spawn weight of AbyssMinionAddedPhysicalDamageJewel6
P  = 39525   total prefix weight available
S  = 24000   total suffix weight available
```

Given a **2-affix** roll, the chance the prefix is the target works out to exactly `w/P`, and
the intermediate cancellation is worth showing because it is why the naive number is not wrong:

```
P(target | 2 affixes)
  = P(1st draw is a prefix) · P(it is the target | prefix)      [2nd draw must then be a suffix]
  + P(1st draw is a suffix) · P(2nd draw is the target)         [2nd draw is from prefixes only]
  = P/(P+S) · w/P  +  S/(P+S) · w/P
  = w/P · (P+S)/(P+S)
  = w/P = 175/39525 = 0.442758%      ← this is the 0.443% in the ticket
```

Given a **1-affix** roll, the single mod is drawn from the whole pool:

```
P(target | 1 affix) = w/(P+S) = 175/63525 = 0.275482%
```

So per *alteration*:

```
P(hit) = 0.5 · 0.442758% + 0.5 · 0.275482% = 0.359120%      → 1 in 278
```

which equals `P(the roll has a prefix at all) · w/P = 0.81110 · 0.442758%` — the two framings
agree, as they must.

**Conclusion on 0.443%.** It is exactly `175/39525`, i.e. the probability *given that the roll
produced a prefix*. It is the number Craft-of-Exile-style tools show, and it is the right number
for "how good is this prefix relative to the other prefixes". It is **not** the per-click hit
rate: about 19% of alterations produce a suffix-only magic item with no prefix at all, so the
real per-click rate is **0.359%**. Also note 0.443% pins the item level at **≥ 86**; at ilvl 83
the same figure is 0.449% (per-click 0.368%).

| ilvl | prefix W | suffix W | `w/P` | per alteration | expected alts | median alts |
| --- | --- | --- | --- | --- | --- | --- |
| 83 | 38950 | 22100 | 0.4493% | 0.3680% | 272 | 188 |
| 84–85 | 39125 | 24000 | 0.4473% | 0.3623% | 276 | 191 |
| ≥ 86 | 39525 | 24000 | 0.4428% | 0.3591% | 278 | 193 |

At ilvl 86: P(at least one hit) is 30% by 100 alts, 51% by 200, 66% by 300, 84% by 500, 97% by
1000. The map's "hundreds of clicks are normal" is accurate.

---

## Source comparison

| source | machine-readable | current | trust | verdict |
| --- | --- | --- | --- | --- |
| **RePoE — `repoe-fork`** ([site](https://repoe-fork.github.io/), [repo](https://github.com/repoe-fork/repoe)) | yes, plain JSON over HTTPS | version.txt `3.29.1.2.2` = live patch | GGG GGPK via PyPoE + poe-tool-dev `dat-schema`; documented schemas in `RePoE/docs/` | **use this** |
| RePoE — `brather1ng/RePoE` (original) | yes | **no**, last push 2022-09-06 | same lineage, abandoned | do not use |
| **poedb.tw** | *yes, better than expected* — see below | current | GGG-datamined, independent toolchain | **use as the cross-check**, not the primary |
| pathofexile.com **trade API** `/api/trade/data/stats` | yes, official JSON | current | GGG official | authoritative for **stat ids and `#`-placeholder texts**; carries **no tiers, ranges, ilvls or weights** |
| **PyPoE** ([Project-Path-of-Exile-Wiki/PyPoE](https://github.com/Project-Path-of-Exile-Wiki/PyPoE)) | it's a library, not a dataset | maintained | highest — it reads the GGPK | only relevant if you ever need a field RePoE doesn't export. Requires a game install |
| **poe-tool-dev** ([dat-schema](https://github.com/poe-tool-dev/dat-schema), [latest-patch-version](https://github.com/poe-tool-dev/latest-patch-version)) | schema + a version string | dat-schema pushed 2026-08-04 | the schema RePoE is generated from | not a mod dataset. **`latest-patch-version` is the staleness oracle** |
| **poe.ninja** | yes (prices API) | current | community | irrelevant — prices, not mod definitions |
| Awakened PoE Trade `stats.ndjson` | yes | tracks releases | derived from RePoE | useful *reference implementation* of the matcher table, not a source of tiers |

### poedb.tw is scrapeable, and cleanly

There is no documented JSON endpoint, but `https://poedb.tw/us/Ghastly_Eye_Jewel` is not
JS-rendered data behind an API — the whole dataset is **inlined in the HTML** as the argument to
a `new ModsView({...})` call, with Mustache templates rendering it client-side. One `curl` plus
`html.find('new ModsView(')` and `json.loads` yields a structured object with `normal` (126
prefix+suffix mods), `corrupted`, `delve`, `synthesis` and 30-odd other pools, each row carrying
`Name`, `Level`, `ModGenerationTypeID` (1=prefix, 2=suffix), `DropChance` (= spawn weight),
`ModFamilyList` (= group), tags, and the mod id in a `hover` field. `robots.txt` is
`User-agent: * / Allow: /`. No rate limit was hit; no licence is stated, which is the reason not
to depend on it structurally.

I used it exactly that way — as an **independent verification** of the RePoE numbers. Result:
**identical**. Same 66 prefixes and 60 suffixes, same mod ids, same levels, same weights,
including `Flaring / Level 83 / DropChance 175 / (23—26) to (33—39)`. Two independently-generated
datamines agreeing on every one of 126 rows is about as good as this gets without reading the
GGPK yourself. The raw slice is committed at
[`data/raw/poedb-modsview.ghastly-eye-jewel.json`](../../data/raw/poedb-modsview.ghastly-eye-jewel.json).

### Which RePoE files, and how they compose

Fetched from `https://repoe-fork.github.io/<file>` (also at
`raw.githubusercontent.com/repoe-fork/repoe-fork.github.io/master/data/<file>`):

- **`base_items.json`** — `Metadata/Items/Jewels/JewelAbyssSummoner` → `name: "Ghastly Eye
  Jewel"`, `item_class: AbyssJewel`, `domain: abyss_jewel`, and the crucial
  `tags: ["not_for_sale", "abyss_jewel_summoner", "abyss_jewel", "default"]`.
- **`mods.json`** (34 MB) — every mod. Per mod: `domain`, `generation_type`, `groups`,
  `required_level`, `stats[{id,min,max}]`, `spawn_weights[{tag,weight}]`, `name`, `text`.
  `spawn_weights` is **order-sensitive**: the first entry whose tag the item has wins; weight 0
  means cannot spawn. For the target mod that is
  `[{abyss_jewel_summoner: 175}, {default: 0}]` — i.e. Ghastly-Eye-only.
- **`mods_by_base.json`** (21 MB) — the pool already resolved per base-tag-set, keyed
  `"Abyss Jewels" → "not_for_sale,abyss_jewel_summoner,abyss_jewel,default" → mods → {prefix,
  suffix, corrupted, delve_prefix, delve_suffix} → group → {mod_id: weight}`. This is the file
  that makes the job easy. I re-derived the same pool independently by scanning `mods.json` and
  applying the `spawn_weights` rule by hand: **identical, 66/66 and 60/60, zero weight
  disagreements**. So `mods_by_base.json` is trustworthy and there is no need to reimplement
  tag resolution.
- **`stat_translations.json`** (13 MB) — `ids[] → English[{string, format, index_handlers,
  condition}]` plus `trade_stats[]`. This is the rendered-text bridge, in both directions.
- `mod_types.json`, `tags.json`, `item_classes.json` — not needed for this base beyond the
  display name `"Abyss Jewels"`.

### Staleness

Two things to know:

1. **The GitHub Pages deploy lags the repo.** When captured, `repoe-fork.github.io/version.txt`
   served `3.29.0.4.2` while `master` held `3.29.1.2.2`. I rebuilt from `master` via
   `raw.githubusercontent.com` and diffed: **byte-identical output** for this base, so the lag
   was harmless here. But the generator should read from `master`, not Pages.
2. **`poe-tool-dev/latest-patch-version/latest.txt` is the oracle.** It read `3.29.1.2.2`,
   matching the RePoE export exactly. A one-line check — "does our stamped `game_version` equal
   `latest.txt`?" — tells you whether the committed data needs regenerating, without diffing
   75 MB.

Licensing: `repoe-fork/repoe` reports `NOASSERTION` on GitHub (i.e. a licence file exists but
isn't SPDX-recognised). The data is derived from GGG's game files either way, so treat the whole
category as "community-normalised game data, fine for a personal tool, don't redistribute as a
product". poedb states no licence at all.

---

## Rendered text → mod identity → tier

This is a solved problem, and PoE 1 solves most of it *for* you.

### Primary path: Alt+Ctrl+C, not Ctrl+C

With **Advanced Mod Descriptions** enabled in the game's UI options, copying an item with
**Alt+Ctrl+C** (or `<advanced-desc-keybind>+Ctrl+C` if rebound) emits a mod-info header line
before each mod, and inlines each roll's own bounds. Real captured example of a magic jewel
(from [bigbes/lootfilter](https://github.com/bigbes/lootfilter/blob/master/itemparser/assets/jewel-magic.txt)):

```
Item Class: Jewels
Rarity: Magic
Volleying Viridian Jewel of Atrophy
--------
Item Level: 81
--------
{ Prefix Modifier "Volleying" (Tier: 1) — Attack, Speed }
8(6-8)% increased Attack Speed with Bows
{ Suffix Modifier "of Atrophy" (Tier: 1) — Damage, Chaos }
+8(6-8)% to Chaos Damage over Time Multiplier
--------
```

So for the target the clipboard would read, verbatim:

```
{ Prefix Modifier "Flaring" (Tier: 1) — Damage, Physical, Minion }
Minions deal 24(23-26) to 31(33-39) additional Physical Damage
```

That block hands you **prefix-vs-suffix, the affix name, the tier number, the roll, and the
roll's own bounds**. The hit test collapses to `generation == 'prefix' && name == 'Flaring'`, or
`tier <= N` for a tier-range target. No data file required at read time at all — the file is for
the *picker*, for validation, and for the probability display.

Craft of Exile requires exactly this (["Make sure you are using ALT+CTRL+C when copying your
item in the game as plain CTRL+C will not work"](https://www.craftofexile.com/faq)), which is a
good sign that it is the community-standard mechanism rather than an accident.

The canonical parser is [Awakened PoE Trade](https://github.com/SnosMe/awakened-poe-trade),
`renderer/src/parser/advanced-mod-desc.ts`. Its approach, worth copying wholesale:

- `isModInfoLine(line)` = `line.startsWith('{') && line.endsWith('}')`.
- `groupLinesByMod()` walks the block, attaching each stat line to the preceding `{…}` header.
- Strip the braces, split on the em-dash `—` into `type-and-name`, `tags`, `increased`,
  then apply
  `/^(?<type>[^"]+)(?:\s+"(?<name>[^"]*)")?(?:\s+\(Tier: (?<tier>\d+)\))?(?:\s+\(Rank: (?<rank>\d+)\))?$/`
  (from `renderer/public/data/en/client_strings.js`). `type` is one of the literals
  `Prefix Modifier`, `Suffix Modifier`, `Implicit Modifier`, `Master Crafted Prefix Modifier`,
  `Fractured Prefix Modifier`, … — note that **fractured and crafted mods carry their own type
  strings**, which is how you avoid treating a fractured prefix as a fresh roll.
- The `— #% increased` third segment is the Synthesis/Cluster roll multiplier. Irrelevant for
  abyss jewels; don't model it.

**Two more free signals in the same clipboard text**, both useful as cross-checks:

- **`Item Level: N`** — needed anyway to validate the target is reachable (T1 needs ≥ 83) and to
  pick the right row of `pool_totals_by_ilvl`.
- **The item's own name.** A magic item is named `<prefix affix> <base> of <suffix affix>`. All
  **66 prefix affix names on this base are unique**, so `"Flaring Ghastly Eye Jewel …"` alone
  identifies mod *and* tier — no advanced descriptions needed. (3 of 57 suffix names repeat
  across tiers within a group — `of Stifling`, `of Distraction`, `of Delaying` — so suffixes
  need the roll to disambiguate. Irrelevant for v1's prefix target.) This makes a very cheap,
  very robust secondary check: agree on `name`, `tier`, and roll-in-range, or refuse to latch.

### Fallback path: plain Ctrl+C, no advanced descriptions

If the setting is off, you get bare lines: `Minions deal 24 to 31 additional Physical Damage`,
no tier, no bounds, no prefix/suffix marker. Recovering identity and tier is then:

1. **Normalise** the line: replace every match of
   `[+-]?\d+(?:\.\d+)?(?:\((?<lo>[^)-]*)-(?<hi>[^)]+)\))?` with `#`, capturing the values.
   Note the sign is absorbed into `#`, so the key is `#% to Chaos Resistance`, not
   `+#% to Chaos Resistance` — this matches Awakened PoE Trade's `matchers[].string`
   convention exactly, which is a useful compatibility property.
2. **Look up** `#`-string → mod group in a prebuilt index. Our `match_lines[].match_string`
   values are precisely that index, generated from `stat_translations.json` by substituting
   `{0}`, `{1}` … with `#` in the `English[].string` whose `condition` matches the roll range.
   On this base the index is collision-free.
3. **Resolve the tier** by testing the parsed roll against each tier's
   `display_min`/`display_max`, and rejecting tiers with `required_ilvl > itemLevel`. On this
   base the ranges never overlap, so exactly one tier matches.
4. **Prefix vs suffix** comes from which pool the group was found in, plus the affix name in the
   item title.

Existing libraries that already do all of this, in case v1 wants to borrow rather than build:

- **Awakened PoE Trade** — `renderer/src/parser/stat-translations.ts` is the reference. It
  generates a combination-space of `#`-placeholder variants (`PLACEHOLDER_MAP`) because some
  translations bake a literal value into the string, then does an exact-map lookup
  (`STAT_BY_MATCH_STR_V2`) and resolves same-text-different-stat collisions with per-group
  strategies (`select`, `trivial-merge`, `percent-merge`, `flag-merge`). Its compiled table,
  `renderer/public/data/en/stats.ndjson`, is a good model for the matcher index shape:
  `{"ref": "...", "matchers": [{"string": "..."}], "trade": {"ids": {...}}}`.
- **XileHUD/poe_overlay** — `src/main/item-parser.ts` does the same header parsing more crudely
  (`/^\{.*\bModifier\b.*\}$/i` plus a `(Tier: (\d+))` capture); useful as a second opinion.
- **kalandralang** and **PoeCraftLib** for the *generation* side (mod pools, weights, currency
  semantics) rather than parsing.

None of that generality is needed for one base with 126 mods and a collision-free index. The
recommendation is: implement the advanced-desc path as primary, the `#`-normalisation path as
fallback, and require **both** to agree before latching the click gate.

---

## Alteration mechanics that bound the model

Consolidated, since these are the constraints later tickets will encode:

- **Magic = at most 1 prefix + 1 suffix.** A magic item cannot carry two prefixes, so "did the
  prefix hit" is a single-slot question. There is no "which of my prefixes" ambiguity in v1.
- **An alteration produces 1 or 2 affixes** (50/50), so a roll can legitimately have *no
  prefix*. The gate logic must treat "no prefix line at all" as a normal non-hit, not a parse
  failure.
- **Item level gates tiers, one-directionally.** ilvl ≥ `required_ilvl`. Higher ilvl never
  removes a tier, it only adds — which is why higher ilvl slightly *lowers* the per-roll chance
  of a specific top-tier mod (bigger denominator). T1 needs ilvl ≥ 83.
- **Tier numbering is ilvl-independent.** `(Tier: 1)` is Tier 1 of the base's ladder even on an
  ilvl-1 item where T1 cannot spawn. Do not renumber tiers by what's reachable.
- **Repeats are possible.** Rolls are i.i.d.; the same mod, even the same roll, can appear
  twice in a row. Any "did it change" heuristic for detecting a new roll is unsafe — compare
  full item text, or better, drive off the click event.
- **One mod per group.** Mods in the same `groups` entry are mutually exclusive on an item. On
  this base prefix groups and suffix groups are disjoint, so for magic items this constraint is
  never binding — the 2-affix probability factorises cleanly, which is why the derivation above
  is exact rather than approximate.
- **Alteration requires a magic item.** Transmute first; a Scour returns it to normal.
- Corrupted and delve mods (`non_alteration_pools`, 21 + 18 mods) can never appear from an
  alteration. They're in the file so the matcher can recognise and ignore them, e.g. on a
  corrupted jewel the user accidentally loads.

---

## Reproducing this

The generator is committed: [`scripts/build-mod-pool.py`](../../scripts/build-mod-pool.py). Its
docstring has the exact `curl` commands for the ~75 MB of upstream (deliberately not committed),
including the one-liner that lifts the poedb dataset out of the page's `new ModsView({...})` call.
Then `python3 scripts/build-mod-pool.py`.

What it does: read the pool out of `mods_by_base.json` for the base's tag set, join each mod id
into `mods.json`, sort each group by `required_level` descending for tier order, and build
`match_lines` by greedily covering the mod's non-zero stat ids with `stat_translations.json`
entries (longest first), picking the `English[]` entry whose `condition` matches the roll range.

It then refuses to produce output unless three self-checks pass, which are the reason to trust
the result:

1. The pool matches a **from-scratch scan of `mods.json`** applying the `spawn_weights`
   first-matching-tag rule by hand — i.e. `mods_by_base.json` is not taken on faith. 66/66 and
   60/60, zero weight disagreements.
2. The derived `display_min`/`display_max` **reproduce the numbers in RePoE's own rendered
   `text`** field for all 126 tiers.
3. Roll → tier is unambiguous (no overlapping tier ranges) and rendered text → group is unique
   (67 collision-free match strings).

The one thing still missing is a CI job comparing the stamped `game_version` against
`poe-tool-dev/latest-patch-version/latest.txt`.

---

## Open questions

- **The 50/50 magic affix count is the one unverified number in the chain.** It is
  community-reverse-engineered and consistent across two simulators, but I found no GGG
  statement and no datamined table. A 2017 forum thread reports 50 hand-rolled samples as
  "~25% prefix, 25% suffix, 50% double", which is in the right ballpark but too small to
  distinguish 50/50 from, say, 40/60. **This only affects the displayed probability, never the
  hit detection** — so it is safe to ship 0.359% with a caveat, and the app itself can settle it
  empirically: log affix counts across a real alt-spam session and the answer falls out after a
  few hundred rolls. That's a nicer resolution than more searching.
- **Does `(Tier: N)` ever disagree with the RePoE-derived ladder?** They agree for this base
  (poedb renders the same tier column from the same rule). Bases where a group has tiers that
  can't spawn on that base are where a mismatch would show up. Not a v1 risk; worth an assertion
  if a second base is ever added.
- **Whether the game emits `24(23-26)` for *every* mod type** or only some. Every fixture I
  found does, and Awakened PoE Trade's parser treats the bounds as optional, which is the safe
  assumption to copy.
- **Abyss jewel implicits.** `implicits: []` in `base_items.json` — a Ghastly Eye Jewel has no
  implicit, so there is no implicit block to skip in the clipboard text. Confirmed, but only for
  this base.
- **Localisation.** Everything here is English-only. `stat_translations.json` ships all 10
  languages and `client_strings.js` shows the mod-header literals are translated, so a non-English
  client would break the matcher. Out of scope, but the data model should not make it impossible.
