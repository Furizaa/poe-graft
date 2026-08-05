//! Loading the tier data, and putting it on the wire.
//!
//! `crates/core` embeds no data — [`ModPool::from_json`] is the only way in — so the file has to reach
//! the installed app as a Tauri resource and be read at startup. That was the owner's call in
//! [#19](https://github.com/Furizaa/poe-graft/issues/19), and it is why `bundle.resources` in
//! `tauri.conf.json` is load-bearing: unwired, the app runs with no tier data at all and cannot assess
//! a single Read.
//!
//! **Failing to load is not survivable, and is not hidden.** There is no fallback tier table and there
//! must never be one: guessing at the numbers is precisely the mistake that would let an over-roll
//! through. The app comes up, says so in the window and in the log, and refuses to arm.

use std::sync::Arc;

use poe_graft_core::{Generation, ModPool, Odds};
use serde::Serialize;
use tauri::path::BaseDirectory;
use tauri::Manager;

/// Where the tier data lands in the bundle. **Must match the target path in
/// `tauri.conf.json > bundle > resources`** — Tauri resolves resources by exactly the string used
/// there.
const POOL_RESOURCE: &str = "data/ghastly-eye-jewel.json";

/// Read `data/ghastly-eye-jewel.json`.
///
/// Returns the pool and the path it came from, because "which file is this actually running?" is a
/// question the gaming PC has no other way to answer.
pub fn load(app: &tauri::AppHandle) -> Result<(Arc<ModPool>, String), String> {
    let (json, from) = read(app)?;
    let pool = ModPool::from_json(&json).map_err(|err| format!("{from}: {err}"))?;
    Ok((Arc::new(pool), from))
}

fn read(app: &tauri::AppHandle) -> Result<(String, String), String> {
    let resource = app
        .path()
        .resolve(POOL_RESOURCE, BaseDirectory::Resource)
        .map_err(|err| format!("could not resolve the resource directory: {err}"))?;

    let error = match std::fs::read_to_string(&resource) {
        Ok(json) => return Ok((json, resource.display().to_string())),
        Err(err) => format!("could not read {}: {err}", resource.display()),
    };

    // `pnpm tauri dev` on the development machine does not necessarily stage bundle resources next to
    // the debug binary, and a dev loop that cannot open its own window is no dev loop. Release builds
    // get no fallback: a binary that quietly reads its safety-critical data from somewhere unexpected
    // is worse than one that refuses to start.
    #[cfg(debug_assertions)]
    {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("data")
            .join("ghastly-eye-jewel.json");
        if let Ok(json) = std::fs::read_to_string(&source) {
            return Ok((json, format!("{} (development fallback)", source.display())));
        }
    }

    Err(error)
}

// ── The wire ────────────────────────────────────────────────────────────────────────────────────

/// The mod pool as the window sees it. Sent once at startup; it does not change.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModPoolDto {
    /// `Ghastly Eye Jewel`.
    pub base_name: String,
    /// `Abyss Jewels`.
    pub item_class: String,
    /// Every Mod Group that can roll on the Base, prefixes then suffixes.
    pub groups: Vec<ModGroupDto>,
}

/// One Mod Group, for the Target Mod picker.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModGroupDto {
    /// The stable identifier a Target Mod is stored as — `MinionAddedPhysicalDamage`.
    pub id: String,
    /// `prefix` or `suffix`.
    pub generation: String,
    /// The rendered lines, values replaced by `#`.
    ///
    /// **This is the label, not an affix name.** A group's display name is per tier — the group behind
    /// the map's target is `Annealed` at T4 and `Flaring` at T1 — so naming a group by any one of them
    /// would be a lie at every other tier.
    pub lines: Vec<String>,
    /// Every tier, best first.
    pub tiers: Vec<ModTierDto>,
}

/// One tier, so the human can see what a Tier Threshold actually buys.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModTierDto {
    /// 1 is the best.
    pub tier: u8,
    /// What the game calls *this* tier of the group.
    pub affix_name: String,
    /// The lowest item level this tier can spawn on. The map's target needs 83.
    pub required_ilvl: u32,
    /// The bands, in the order the values appear in the rendered line.
    pub bands: Vec<[f64; 2]>,
}

/// What a Tier Threshold costs, for the window.
///
/// Computed in Rust rather than in the frontend on purpose: ADR 0001 keeps domain logic on this side of
/// the seam, and the arithmetic is pinned to `docs/research/mod-tier-data.md` by
/// `crates/core/tests/odds.rs`. The window formats these numbers and does not derive them — with one
/// stated exception, `cumulative`, which the frontend recomputes per render because it changes with every
/// Roll; it is `1-(1-perClick)^n` over a probability this side already established.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OddsDto {
    /// Summed spawn weight at or better than the threshold, reachable at this item level.
    pub weight: u32,
    /// The chance **given** the roll produced an affix of this generation — #9's `0.443%`.
    pub conditional: f64,
    /// The real per-click hit rate.
    pub per_click: f64,
    /// `1 in N` rolls. Absent when nothing that good can spawn.
    pub one_in: Option<u32>,
    /// The fewest Rolls at which a Hit is more likely than not.
    pub median_rolls: Option<u32>,
    /// Nothing at or better than the threshold can spawn at this item level.
    pub impossible: bool,
}

impl OddsDto {
    /// Put an [`Odds`] on the wire.
    pub fn of(odds: &Odds) -> Self {
        Self {
            weight: odds.weight,
            conditional: odds.conditional,
            per_click: odds.per_click,
            one_in: odds.one_in(),
            median_rolls: odds.median_rolls(),
            impossible: odds.is_impossible(),
        }
    }
}

impl ModPoolDto {
    /// Flatten a pool for the window.
    pub fn of(pool: &ModPool) -> Self {
        Self {
            base_name: pool.base_name().to_string(),
            item_class: pool.item_class().to_string(),
            groups: pool
                .groups()
                .iter()
                .map(|group| ModGroupDto {
                    id: group.id().to_string(),
                    generation: match group.generation() {
                        Generation::Prefix => "prefix".into(),
                        Generation::Suffix => "suffix".into(),
                    },
                    lines: group.match_strings().to_vec(),
                    tiers: group
                        .tiers()
                        .iter()
                        .map(|tier| ModTierDto {
                            tier: tier.tier(),
                            affix_name: tier.affix_name().to_string(),
                            required_ilvl: tier.required_ilvl(),
                            bands: tier
                                .bands()
                                .iter()
                                .map(|band| [band.min(), band.max()])
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
