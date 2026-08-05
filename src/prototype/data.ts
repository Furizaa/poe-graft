/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * The tier data, read straight from `data/ghastly-eye-jewel.json`, and the odds arithmetic the
 * variants display.
 *
 * Two reasons this exists rather than using `getModPool()`:
 *
 * 1. **The seam does not carry `spawn_weight`.** `ModGroup` has the tiers, the bands and the
 *    required item level, but not the weights — so odds cannot be computed from the Rust side as it
 *    stands. A shipped odds display therefore needs the weights added to `ModPool`, which is a
 *    change to `crates/core` and to `src/api.ts`, not a layout decision. Noted on #9.
 * 2. **It makes the prototype run in a plain browser.** `pnpm dev` gives no `invoke()`, so
 *    `getModPool()` rejects; falling back to this file means the variants can be flipped through in
 *    a real browser with a URL bar and devtools, which is a much better loop than the Tauri webview.
 */
import raw from "../../data/ghastly-eye-jewel.json?raw";
import type { ModGroup, ModPool } from "../api";

type RawStat = { display_min: number; display_max: number };

type RawTier = {
  tier: number;
  affix_name: string;
  required_ilvl: number;
  spawn_weight: number;
  stats: RawStat[];
};

type RawGroup = {
  group: string;
  generation_type: "prefix" | "suffix";
  match_lines: { match_string: string }[];
  tiers: RawTier[];
};

/** A mod from a pool an Orb of Alteration cannot reach. Flat — no tiers, no bands. */
type RawFlatMod = {
  mod_id: string;
  group: string;
  affix_name: string;
  generation_type: string;
  required_ilvl: number;
  text: string;
};

type RawFile = {
  base: { name: string; item_class_display: string };
  pool_totals_by_ilvl: { ilvl: number; prefix_weight: number; suffix_weight: number }[];
  affix_count_odds: { magic: Record<string, number> };
  prefixes: RawGroup[];
  suffixes: RawGroup[];
  non_alteration_pools: Record<string, RawFlatMod[]>;
};

const file = JSON.parse(raw) as RawFile;

const rawGroups = [...file.prefixes, ...file.suffixes];

/** Group id → the raw group, because the seam's `ModGroup` has no weights. */
export const rawById = new Map(rawGroups.map((g) => [g.group, g]));

/** The same shape `getModPool()` returns, derived locally so this runs without Tauri. */
export const localPool: ModPool = {
  baseName: file.base.name,
  itemClass: file.base.item_class_display,
  groups: rawGroups.map(
    (g): ModGroup => ({
      id: g.group,
      generation: g.generation_type,
      lines: g.match_lines.map((l) => l.match_string),
      tiers: g.tiers.map((t) => ({
        tier: t.tier,
        affixName: t.affix_name,
        requiredIlvl: t.required_ilvl,
        bands: t.stats.map((s): [number, number] => [s.display_min, s.display_max]),
      })),
    }),
  ),
};

/**
 * The item levels at which the pool totals change — and therefore at which the odds change.
 *
 * Worth surfacing in the UI: **the odds are item-level dependent and today's window has no item
 * level in it.** The recorded 0.3591% is the ilvl ≥ 86 figure; the same target on an ilvl 83 jewel
 * is 0.3680%. Pre-arm the window has to assume a level; mid-session it does not have to, because the
 * parser already reads `Item Level:` off the Item Text.
 */
export const ilvlBreakpoints = file.pool_totals_by_ilvl.map((r) => r.ilvl);

/**
 * Mods this Base can carry that **an Orb of Alteration can never produce.**
 *
 * The picker needs these for a reason that is not cosmetic. Grouping mods by type is only half a
 * feature; the other half is that a target an alteration cannot roll is a craft that can never hit, and
 * a human who cannot find `increased Area of Effect` in the list has no way to tell whether the search
 * is broken or the mod is unreachable. So they are listed, grouped, and **not selectable** — the absence
 * explains itself.
 *
 * Two wrinkles the data forced:
 *
 * - These entries are **flat** — a `mod_id`, one `text`, no tiers and no bands — so they can never be a
 *   `Target { group_id, tier_threshold }` even in principle.
 * - Some group ids appear in **both** pools: `AvoidIgnite`, `AvoidStun`, `ChanceToAvoidBleeding`,
 *   `ChanceToAvoidFreezeAndChill`, `ChanceToAvoidPoison` and `PercentDamageGoesToMana` are alteration
 *   mods *and* corrupted/delve mods. Listing those as unreachable would be a lie, so anything present in
 *   the alteration pool is filtered out here and stays selectable.
 */
const CATEGORY_NAMES: Record<string, string> = {
  corrupted: "Corrupted — Vaal Orb only",
  delve_prefix: "Delve prefix — fossil only",
  delve_suffix: "Delve suffix — fossil only",
};

export type UnreachableMod = {
  id: string;
  /** Which pool it came from, in words. */
  category: string;
  /** The rendered line, values intact — there is no `#` form for these. */
  line: string;
  requiredIlvl: number;
};

const alterationIds = new Set(rawGroups.map((g) => g.group));

export const unreachableMods: UnreachableMod[] = Object.entries(file.non_alteration_pools)
  .flatMap(([pool, mods]) =>
    mods
      .filter((mod) => !alterationIds.has(mod.group))
      .map((mod) => ({
        id: `${pool}:${mod.mod_id}`,
        category: CATEGORY_NAMES[pool] ?? pool,
        line: mod.text,
        requiredIlvl: mod.required_ilvl,
      })),
  )
  .sort((a, b) => a.category.localeCompare(b.category) || a.line.localeCompare(b.line));

const totalsFor = (ilvl: number) => {
  let best = file.pool_totals_by_ilvl[0];
  for (const row of file.pool_totals_by_ilvl) if (row.ilvl <= ilvl) best = row;
  return best;
};

/** The 50/50 one-or-two-affix split. Community-derived, not GGG data — say so on screen. */
const magicSplit = () => {
  const weights = file.affix_count_odds.magic;
  const one = weights["1"] ?? 1;
  const two = weights["2"] ?? 1;
  return { one: one / (one + two), two: two / (one + two) };
};

export type Odds = {
  /** Summed spawn weight of every tier at or better than the threshold that can spawn at this ilvl. */
  weight: number;
  /** `w / W` — the chance *given* the roll produced an affix of this generation. #9's 0.443%. */
  conditional: number;
  /** The real per-click hit rate. What a human actually experiences. */
  perClick: number;
  /** `1 / perClick`, rounded. */
  oneIn: number;
  /** Rolls at which you are more likely than not to have hit. */
  median: number;
  /** True when no tier at or better than the threshold can spawn at this item level at all. */
  impossible: boolean;
};

/**
 * Per-click odds of a Hit, matching `docs/research/mod-tier-data.md` exactly.
 *
 * A magic item gets one or two affixes at 50/50, and at most one prefix and one suffix — so with two
 * affixes the wanted generation is drawn for certain, and with one it is drawn only if that
 * generation wins the coin weighted by pool size.
 */
export const odds = (group: ModGroup, threshold: number, ilvl: number): Odds => {
  const rawGroup = rawById.get(group.id);
  const totals = totalsFor(ilvl);
  const own = group.generation === "prefix" ? totals.prefix_weight : totals.suffix_weight;
  const both = totals.prefix_weight + totals.suffix_weight;
  const split = magicSplit();

  const weight = (rawGroup?.tiers ?? [])
    .filter((t) => t.tier <= threshold && t.required_ilvl <= ilvl)
    .reduce((sum, t) => sum + t.spawn_weight, 0);

  const conditional = weight / own;
  const perClick = split.two * conditional + split.one * (weight / both);

  return {
    weight,
    conditional,
    perClick,
    oneIn: perClick > 0 ? Math.round(1 / perClick) : Infinity,
    median: perClick > 0 ? Math.ceil(Math.log(0.5) / Math.log(1 - perClick)) : Infinity,
    impossible: weight === 0,
  };
};

/** `1-(1-p)^n` — the chance of having hit at least once by roll `n`. ADR 0002 removed the roll cap. */
export const cumulative = (perClick: number, rolls: number) =>
  perClick > 0 ? 1 - Math.pow(1 - perClick, rolls) : 0;

export const percent = (p: number, places = 2) =>
  `${(p * 100).toFixed(p * 100 < 1 ? Math.max(places, 3) : places)}%`;

/** `23–26 to 33–39` — the numbers the game will print. Same helper the real panel uses. */
export const bands = (group: ModGroup, tier: number) => {
  const found = group.tiers.find((t) => t.tier === tier);
  if (!found) return "";
  return found.bands.map(([min, max]) => (min === max ? `${min}` : `${min}–${max}`)).join(" to ");
};

/** `Minions deal # to # additional Physical Damage` — never an affix name, which is per tier. */
export const label = (group: ModGroup) => group.lines.join(" · ");

/**
 * The Tier Threshold to carry across a change of Mod Group — 40 of the 66 groups have only a Tier 1,
 * so an uncarried threshold makes most group changes fail. Same rule as `Craft.tsx` after 468ab97.
 */
export const carryThreshold = (group: ModGroup, threshold: number) =>
  Math.min(threshold, Math.max(...group.tiers.map((t) => t.tier)));
