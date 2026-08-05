//! The mod pool: what modifiers a Base can have, and at what tiers.
//!
//! Read from `data/ghastly-eye-jewel.json`, whose provenance is
//! [`docs/research/mod-tier-data.md`](../../../docs/research/mod-tier-data.md). The core embeds no
//! data — `src-tauri` reads the file and hands the JSON in, so there is exactly one copy and it is
//! the one that shipped.
//!
//! A **Mod Group** is never identified by name. Its display name is per tier — the group behind the
//! target is `Annealed` at T4 and `Flaring` at T1 — so the handle is the group's *rendered line*
//! with every rolled value replaced by `#`, which is stable across tiers by construction.

use std::collections::HashMap;

use serde::Deserialize;

/// Why the mod pool could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// The JSON did not have the shape this crate expects.
    Malformed(String),
    /// Two Mod Groups claim the same rendered line, so a parsed mod would match both. Fatal rather
    /// than resolved arbitrarily: the hit test would be guessing which group the human asked for.
    AmbiguousMatchString {
        /// The rendered line both groups claim.
        match_string: String,
        /// The groups that claim it.
        groups: [String; 2],
    },
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(detail) => write!(f, "mod pool is malformed: {detail}"),
            Self::AmbiguousMatchString {
                match_string,
                groups,
            } => write!(
                f,
                "mod pool is ambiguous: {:?} and {:?} both render as {match_string:?}",
                groups[0], groups[1]
            ),
        }
    }
}

impl std::error::Error for DataError {}

/// Whether a mod occupies the Base's prefix slot or its suffix slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    /// A prefix.
    Prefix,
    /// A suffix.
    Suffix,
}

impl Generation {
    /// How the game names this in an Item Text annotation: `Prefix Modifier`.
    pub fn annotation_word(self) -> &'static str {
        match self {
            Self::Prefix => "Prefix",
            Self::Suffix => "Suffix",
        }
    }
}

/// The inclusive band one stat of one tier can roll in, in the units the tooltip prints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    min: f64,
    max: f64,
}

impl Band {
    /// Does a rolled value fall inside this band? Inclusive at both ends.
    pub fn contains(&self, value: f64) -> bool {
        value >= self.min && value <= self.max
    }

    /// The band's lower bound, as printed.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// The band's upper bound, as printed.
    pub fn max(&self) -> f64 {
        self.max
    }
}

/// One tier of one Mod Group.
#[derive(Debug, Clone)]
pub struct ModTier {
    tier: u8,
    affix_name: String,
    required_ilvl: u32,
    spawn_weight: Option<u32>,
    bands: Vec<Band>,
}

impl ModTier {
    /// The tier number: 1 is the best.
    pub fn tier(&self) -> u8 {
        self.tier
    }

    /// The name the game shows for *this tier* of the group — `Flaring`, `Annealed`.
    pub fn affix_name(&self) -> &str {
        &self.affix_name
    }

    /// The lowest item level on which this tier can spawn.
    pub fn required_ilvl(&self) -> u32 {
        self.required_ilvl
    }

    /// How likely this tier is relative to everything else that could roll in its slot.
    ///
    /// Never consulted by a Verdict — a Hit is decided by the numbers, not by how improbable they were.
    /// This exists so the window can say what a Tier Threshold costs in Alterations.
    ///
    /// `None` when the data file does not carry weights. Optional on purpose: odds are informational, so
    /// a file without them must still load and assess Reads — refusing to start over a number no Verdict
    /// consults would be the tail wagging the dog. It is an `Option` rather than a `0` because "we cannot
    /// say" and "this cannot spawn" are different claims and the window shows them differently.
    pub fn spawn_weight(&self) -> Option<u32> {
        self.spawn_weight
    }

    /// The bands, in the order the values appear in the rendered line.
    pub fn bands(&self) -> &[Band] {
        &self.bands
    }

    /// Do these rolled values, in order, all fall inside this tier's bands?
    ///
    /// A different count of values than bands is never a match — it means the line was matched to
    /// the wrong group, which the caller must not paper over.
    pub fn accepts(&self, values: &[f64]) -> bool {
        values.len() == self.bands.len()
            && values
                .iter()
                .zip(&self.bands)
                .all(|(v, band)| band.contains(*v))
    }
}

/// A family of modifiers spanning every tier of it.
#[derive(Debug, Clone)]
pub struct ModGroup {
    id: String,
    generation: Generation,
    match_strings: Vec<String>,
    tiers: Vec<ModTier>,
}

impl ModGroup {
    /// The group's stable identifier — `MinionAddedPhysicalDamage`. Not shown to the human.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Prefix or suffix.
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// Every rendered line this group can produce, values replaced by `#`.
    pub fn match_strings(&self) -> &[String] {
        &self.match_strings
    }

    /// How many tiers the group has on this Base.
    pub fn tier_count(&self) -> usize {
        self.tiers.len()
    }

    /// Every tier, best first.
    pub fn tiers(&self) -> &[ModTier] {
        &self.tiers
    }

    /// One tier by number.
    pub fn tier(&self, tier: u8) -> Option<&ModTier> {
        self.tiers.iter().find(|t| t.tier == tier)
    }
}

/// The Base being crafted, and what it can roll.
#[derive(Debug, Clone)]
pub struct ModPool {
    base_name: String,
    item_class: String,
    implicit_count: usize,
    groups: Vec<ModGroup>,
    by_match: HashMap<String, usize>,
    pool_totals: Vec<PoolTotals>,
    /// Relative weights for a magic item getting one affix versus two.
    magic_affix_split: (u32, u32),
}

/// The total spawn weight available in each slot at an item level.
///
/// Stored as the breakpoints the data file gives, which is what makes the odds item-level dependent:
/// the same Target Mod is 1 in 272 on an item level 83 jewel and 1 in 278 at 86 or above.
#[derive(Debug, Clone, Copy)]
pub struct PoolTotals {
    /// The lowest item level these totals apply from.
    pub ilvl: u32,
    /// Everything that can roll in the prefix slot, summed.
    pub prefix_weight: u32,
    /// Everything that can roll in the suffix slot, summed.
    pub suffix_weight: u32,
}

/// What a Tier Threshold costs, in Alterations.
///
/// **Informational, and deliberately outside every decision the app makes.** No Verdict, Refusal, Halt
/// or Latch consults any of this — a Hit is decided by the numbers on the item, and would be a Hit if it
/// landed on the first click. It exists because [#9](https://github.com/Furizaa/poe-graft/issues/9) asks
/// the window to show the roll count and cumulative probability in place of the roll cap ADR 0002
/// removed, and because a human deciding whether to keep going deserves the real number rather than a
/// remembered one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Odds {
    /// Summed spawn weight of every tier at or better than the threshold that can spawn at this ilvl.
    pub weight: u32,
    /// `weight / own-slot total` — the chance **given** the roll produced an affix of this generation.
    ///
    /// This is the figure quoted as `0.443%` on #9. It is correct as a conditional and misleading as a
    /// per-click rate, so it is carried separately rather than being the headline.
    pub conditional: f64,
    /// The real per-click hit rate: what a human actually experiences per Alteration.
    pub per_click: f64,
}

impl Odds {
    /// Nothing at or better than the threshold can spawn at this item level at all.
    ///
    /// Worth saying on screen: it is the difference between a slow craft and an impossible one.
    pub fn is_impossible(&self) -> bool {
        self.weight == 0
    }

    /// `1 in N` rolls, rounded. `None` when impossible.
    pub fn one_in(&self) -> Option<u32> {
        (self.per_click > 0.0).then(|| (1.0 / self.per_click).round() as u32)
    }

    /// The chance of having hit at least once by roll `n` — `1-(1-p)^n`.
    pub fn cumulative(&self, rolls: u32) -> f64 {
        if self.per_click <= 0.0 {
            return 0.0;
        }
        1.0 - (1.0 - self.per_click).powi(rolls as i32)
    }

    /// The fewest Rolls at which a Hit is more likely than not. `None` when impossible.
    ///
    /// The smallest `n` with `cumulative(n) >= 0.5`, which is a ceiling rather than a rounding. Note that
    /// `docs/research/mod-tier-data.md` lists 188 for the ilvl 83 case where this returns 189: the exact
    /// figure is 188.01, and at 188 Rolls the cumulative chance is 49.997%, just under the half.
    pub fn median_rolls(&self) -> Option<u32> {
        (self.per_click > 0.0 && self.per_click < 1.0)
            .then(|| (0.5f64.ln() / (1.0 - self.per_click).ln()).ceil() as u32)
    }
}

impl ModPool {
    /// Read a pool from `data/ghastly-eye-jewel.json`'s text.
    pub fn from_json(json: &str) -> Result<Self, DataError> {
        let raw: RawPool =
            serde_json::from_str(json).map_err(|e| DataError::Malformed(e.to_string()))?;

        let mut groups = Vec::with_capacity(raw.prefixes.len() + raw.suffixes.len());
        for (generation, raws) in [
            (Generation::Prefix, raw.prefixes),
            (Generation::Suffix, raw.suffixes),
        ] {
            for g in raws {
                groups.push(ModGroup {
                    id: g.group,
                    generation,
                    match_strings: g.match_lines.into_iter().map(|m| m.match_string).collect(),
                    tiers: g
                        .tiers
                        .into_iter()
                        .map(|t| ModTier {
                            tier: t.tier,
                            affix_name: t.affix_name,
                            required_ilvl: t.required_ilvl,
                            spawn_weight: t.spawn_weight,
                            bands: t
                                .stats
                                .into_iter()
                                .map(|s| Band {
                                    min: s.display_min,
                                    max: s.display_max,
                                })
                                .collect(),
                        })
                        .collect(),
                });
            }
        }

        let mut by_match = HashMap::new();
        for (i, group) in groups.iter().enumerate() {
            for ms in &group.match_strings {
                if let Some(previous) = by_match.insert(ms.clone(), i) {
                    return Err(DataError::AmbiguousMatchString {
                        match_string: ms.clone(),
                        groups: [groups[previous].id.clone(), group.id.clone()],
                    });
                }
            }
        }

        let mut pool_totals = raw.pool_totals_by_ilvl;
        pool_totals.sort_by_key(|row| row.ilvl);
        let pool_totals = pool_totals
            .into_iter()
            .map(|row| PoolTotals {
                ilvl: row.ilvl,
                prefix_weight: row.prefix_weight,
                suffix_weight: row.suffix_weight,
            })
            .collect();

        // Read from the file rather than hardcoded, because the file is honest about it: this split is
        // community-derived from two independent crafting simulators and is not in GGG's data. Its own
        // `_note` says so, and the window repeats the caveat.
        let magic = &raw.affix_count_odds.magic;
        let magic_affix_split = (
            magic.get("1").copied().unwrap_or(1),
            magic.get("2").copied().unwrap_or(1),
        );

        Ok(Self {
            base_name: raw.base.name,
            item_class: raw.base.item_class_display,
            implicit_count: raw.base.implicits.len(),
            groups,
            by_match,
            pool_totals,
            magic_affix_split,
        })
    }

    /// The Base's name as the game prints it inside an item's name — `Ghastly Eye Jewel`.
    pub fn base_name(&self) -> &str {
        &self.base_name
    }

    /// The Base's item class as the game prints it — `Abyss Jewels`.
    pub fn item_class(&self) -> &str {
        &self.item_class
    }

    /// How many implicit modifiers the Base has.
    ///
    /// Load-bearing for section awareness: the parser locates the explicit-mod section structurally,
    /// and an implicit section sitting in front of it would shift that. This Base has none, and the
    /// parser refuses to guess rather than assuming it for the next one.
    pub fn implicit_count(&self) -> usize {
        self.implicit_count
    }

    /// Find the Mod Group that renders as this line, values replaced by `#`.
    pub fn group(&self, match_string: &str) -> Option<&ModGroup> {
        self.by_match.get(match_string).map(|&i| &self.groups[i])
    }

    /// Find a Mod Group by its identifier — the handle the human's Target Mod is stored as.
    pub fn group_by_id(&self, id: &str) -> Option<&ModGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Every group, prefixes then suffixes.
    pub fn groups(&self) -> &[ModGroup] {
        &self.groups
    }

    /// The spawn-weight totals in force at an item level.
    ///
    /// The breakpoints are lower bounds, so this is the last row at or below `ilvl`.
    pub fn totals_at(&self, ilvl: u32) -> Option<PoolTotals> {
        self.pool_totals
            .iter()
            .rev()
            .find(|row| row.ilvl <= ilvl)
            .copied()
    }

    /// What it costs, per Alteration, to hit this group at this Tier Threshold on a jewel of this level.
    ///
    /// The model, which reproduces `docs/research/mod-tier-data.md`:
    ///
    /// An Alteration rerolls a magic item, which gets one or two affixes at the file's `magic` weights
    /// (1:1, so 50/50) and can hold **at most one prefix and one suffix**. So with two affixes the wanted
    /// generation is drawn for certain and the group competes only within its own slot; with one affix the
    /// generation itself has to win a coin weighted by the two slot totals first. Hence
    /// `p = P(two)·w/W_own + P(one)·w/(W_prefix+W_suffix)`.
    ///
    /// Tiers requiring a higher item level than the jewel are excluded, because they cannot spawn — which
    /// is what makes a threshold better than the jewel supports come back impossible rather than merely
    /// unlikely.
    ///
    /// `None` only when the group id is unknown or the item level is below every breakpoint.
    pub fn odds(&self, group_id: &str, tier_threshold: u8, ilvl: u32) -> Option<Odds> {
        let group = self.group_by_id(group_id)?;
        let totals = self.totals_at(ilvl)?;

        let own = match group.generation() {
            Generation::Prefix => totals.prefix_weight,
            Generation::Suffix => totals.suffix_weight,
        };
        let both = totals.prefix_weight + totals.suffix_weight;
        if own == 0 || both == 0 {
            return None;
        }

        // A qualifying tier with no weight makes the whole figure unknowable rather than smaller, so this
        // is `None` on the first gap instead of quietly summing what it has.
        let mut weight: u32 = 0;
        for tier in group.tiers() {
            if tier.tier() <= tier_threshold && tier.required_ilvl() <= ilvl {
                weight += tier.spawn_weight()?;
            }
        }

        let (one, two) = self.magic_affix_split;
        let split = f64::from(one) + f64::from(two);
        let p_one = f64::from(one) / split;
        let p_two = f64::from(two) / split;

        let conditional = f64::from(weight) / f64::from(own);
        let per_click = p_two * conditional + p_one * (f64::from(weight) / f64::from(both));

        Some(Odds {
            weight,
            conditional,
            per_click,
        })
    }
}

// ── the JSON shape, mirrored only as far as the core needs it ────────────────────────────────────

#[derive(Deserialize)]
struct RawPool {
    base: RawBase,
    prefixes: Vec<RawGroup>,
    suffixes: Vec<RawGroup>,
    /// Absent in hand-written test pools, and optional for the same reason `spawn_weight` is: without it
    /// there are simply no odds to show, which must not stop the app assessing Reads.
    #[serde(default)]
    pool_totals_by_ilvl: Vec<RawPoolTotals>,
    #[serde(default)]
    affix_count_odds: RawAffixCountOdds,
}

#[derive(Deserialize)]
struct RawPoolTotals {
    ilvl: u32,
    prefix_weight: u32,
    suffix_weight: u32,
}

#[derive(Deserialize, Default)]
struct RawAffixCountOdds {
    /// Keyed by affix count as a string. The sibling `_note` is ignored.
    #[serde(default)]
    magic: HashMap<String, u32>,
}

#[derive(Deserialize)]
struct RawBase {
    name: String,
    item_class_display: String,
    implicits: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawGroup {
    group: String,
    match_lines: Vec<RawMatchLine>,
    tiers: Vec<RawTier>,
}

#[derive(Deserialize)]
struct RawMatchLine {
    match_string: String,
}

#[derive(Deserialize)]
struct RawTier {
    tier: u8,
    affix_name: String,
    required_ilvl: u32,
    #[serde(default)]
    spawn_weight: Option<u32>,
    stats: Vec<RawStat>,
}

#[derive(Deserialize)]
struct RawStat {
    display_min: f64,
    display_max: f64,
}
