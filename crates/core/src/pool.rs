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

        Ok(Self {
            base_name: raw.base.name,
            item_class: raw.base.item_class_display,
            implicit_count: raw.base.implicits.len(),
            groups,
            by_match,
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
}

// ── the JSON shape, mirrored only as far as the core needs it ────────────────────────────────────

#[derive(Deserialize)]
struct RawPool {
    base: RawBase,
    prefixes: Vec<RawGroup>,
    suffixes: Vec<RawGroup>,
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
    stats: Vec<RawStat>,
}

#[derive(Deserialize)]
struct RawStat {
    display_min: f64,
    display_max: f64,
}
