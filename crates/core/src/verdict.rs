//! The hit test: what a Read establishes about the item under the Anchor.
//!
//! **Tier is derived from the rolled numbers, never from the game's `(Tier: N)` annotation.** The
//! annotation is a client setting (Advanced Mod Descriptions) and its affix name is per tier, so
//! depending on it means breaking silently on a machine where the setting is off. It is cross-checked
//! and reported as a [`Diagnostic`]; it never reaches a [`Verdict`].
//!
//! **Ambiguity fails closed.** Values matching no tier, or more than one, are called a `Hit`: the
//! safe direction is always to stop. [#4](https://github.com/Furizaa/poe-graft/issues/4) established
//! that this Base has no overlapping tier ranges, so neither should ever fire — which is exactly why
//! both are tests.
//!
//! This module decides a Verdict and reports what it saw. It does **not** decide to `Halt` — that is
//! the roll cycle's, which also halts on the wrong item and on three consecutive `Unknown` Verdicts
//! ([ADR 0002](../../../docs/adr/0002-roll-cycle-and-hit-latch.md)).

use crate::item::Item;
use crate::pool::ModPool;

/// What a Read establishes about the item currently under the Anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The Target Mod is present at or better than the Tier Threshold. Latch.
    Hit,
    /// The Target Mod is positively absent or out of tier. Only a fresh Miss permits a Roll.
    Miss,
    /// Nothing was established. Never permits a Roll; resolved by a Resync.
    Unknown,
}

/// The Mod Group the human is crafting for, and the worst tier that still counts as success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    group_id: String,
    tier_threshold: u8,
}

impl Target {
    /// A Target Mod: a Mod Group's identifier and the worst acceptable tier.
    pub fn new(group_id: impl Into<String>, tier_threshold: u8) -> Self {
        Self {
            group_id: group_id.into(),
            tier_threshold,
        }
    }

    /// The Mod Group being crafted for.
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    /// The worst tier that still counts as a Hit. Tier 1 is the best, so a Hit needs
    /// `derived <= tier_threshold`.
    pub fn tier_threshold(&self) -> u8 {
        self.tier_threshold
    }
}

/// Something worth saying about a Read, beyond its Verdict.
///
/// These are what the roll cycle turns into a `Halt`, and what the log records so a wrong row in the
/// tier data is discoverable rather than invisible.
#[derive(Debug, Clone, PartialEq)]
pub enum Diagnostic {
    /// The game's annotation names a different tier than the numbers imply. **Our tier data is
    /// wrong.** ADR 0002 makes this a `Halt` with a loud diagnostic.
    AnnotationDisagrees {
        /// The mod line as rendered.
        line: String,
        /// The tier the rolled values imply — what the Verdict used.
        derived: u8,
        /// The tier the game claims.
        annotated: u8,
    },
    /// The target mod carried no annotation, so no cross-check was possible. Expected whenever
    /// Advanced Mod Descriptions is off; not a problem, and never a `Halt`.
    AnnotationAbsent {
        /// The mod line as rendered.
        line: String,
    },
    /// The rolled values fall inside no tier of the group. Fails closed to a `Hit`.
    NoTierMatched {
        /// The mod line as rendered.
        line: String,
        /// The values read from it.
        values: Vec<f64>,
    },
    /// The rolled values fall inside more than one tier. Fails closed to a `Hit`.
    ManyTiersMatched {
        /// The mod line as rendered.
        line: String,
        /// Every tier that accepted the values.
        tiers: Vec<u8>,
    },
    /// A line in the explicit-mod section that the pool does not recognise. The app is reading an
    /// item it does not understand, so it establishes nothing.
    UnrecognisedLine {
        /// The line as rendered.
        line: String,
    },
    /// The item is not the Base the pool describes, so it cannot be assessed at all.
    NotTheBase {
        /// The name the game printed.
        found: String,
    },
    /// The Target Mod names a Mod Group this pool does not have. A configuration fault, not a roll.
    UnknownTarget {
        /// The identifier that was asked for.
        group_id: String,
    },
}

/// A Verdict, the tier behind it, and everything else worth reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Assessment {
    verdict: Verdict,
    tier: Option<u8>,
    diagnostics: Vec<Diagnostic>,
}

impl Assessment {
    /// The Verdict. This is what the roll cycle acts on.
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The tier derived from the rolled numbers, when the Target Mod was present and unambiguous.
    pub fn tier(&self) -> Option<u8> {
        self.tier
    }

    /// What else was seen. Logged; some of it makes the cycle `Halt`.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Does anything here mean the app can no longer trust its own tier data?
    ///
    /// The roll cycle's wrong-item and three-Unknowns halts are separate; this is only the half the
    /// hit test can see.
    pub fn halt_worthy(&self) -> bool {
        self.diagnostics.iter().any(|d| {
            matches!(
                d,
                Diagnostic::AnnotationDisagrees { .. }
                    | Diagnostic::UnrecognisedLine { .. }
                    | Diagnostic::NotTheBase { .. }
                    | Diagnostic::UnknownTarget { .. }
            )
        })
    }
}

/// Assess a parsed item against a Target Mod.
pub fn assess(item: &Item, target: &Target, pool: &ModPool) -> Assessment {
    let mut diagnostics: Vec<Diagnostic> = item
        .unrecognised()
        .iter()
        .map(|line| Diagnostic::UnrecognisedLine { line: line.clone() })
        .collect();

    if !item.base_matches_pool() {
        diagnostics.push(Diagnostic::NotTheBase {
            found: item.identity().base_name().to_string(),
        });
        return Assessment {
            verdict: Verdict::Unknown,
            tier: None,
            diagnostics,
        };
    }

    let Some(group) = pool.group_by_id(target.group_id()) else {
        diagnostics.push(Diagnostic::UnknownTarget {
            group_id: target.group_id().to_string(),
        });
        return Assessment {
            verdict: Verdict::Unknown,
            tier: None,
            diagnostics,
        };
    };

    let Some(found) = item.mod_of_group(target.group_id()) else {
        // Positively absent: the item parsed, every line was recognised, and this group is not among
        // them. That is a Miss — the one Verdict that permits spending an Alteration.
        let verdict = if diagnostics.is_empty() {
            Verdict::Miss
        } else {
            Verdict::Unknown
        };
        return Assessment {
            verdict,
            tier: None,
            diagnostics,
        };
    };

    let line = found.lines().join("\n");
    let matching: Vec<u8> = group
        .tiers()
        .iter()
        .filter(|t| t.accepts(found.values()))
        .map(|t| t.tier())
        .collect();

    let derived = match matching.as_slice() {
        [tier] => *tier,
        [] => {
            diagnostics.push(Diagnostic::NoTierMatched {
                line,
                values: found.values().to_vec(),
            });
            return Assessment {
                verdict: Verdict::Hit,
                tier: None,
                diagnostics,
            };
        }
        many => {
            diagnostics.push(Diagnostic::ManyTiersMatched {
                line,
                tiers: many.to_vec(),
            });
            return Assessment {
                verdict: Verdict::Hit,
                tier: None,
                diagnostics,
            };
        }
    };

    // The cross-check, after the tier is already decided. Reordering this so the annotation could
    // influence `derived` is the one change this module must never accept.
    match found.annotation() {
        Some(annotation) if annotation.tier() != derived => {
            diagnostics.push(Diagnostic::AnnotationDisagrees {
                line,
                derived,
                annotated: annotation.tier(),
            });
        }
        Some(_) => {}
        None => diagnostics.push(Diagnostic::AnnotationAbsent { line }),
    }

    let verdict = if derived <= target.tier_threshold() {
        Verdict::Hit
    } else if diagnostics
        .iter()
        .any(|d| matches!(d, Diagnostic::UnrecognisedLine { .. }))
    {
        Verdict::Unknown
    } else {
        Verdict::Miss
    };

    Assessment {
        verdict,
        tier: Some(derived),
        diagnostics,
    }
}
