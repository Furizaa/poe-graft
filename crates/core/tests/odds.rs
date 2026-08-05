//! The odds arithmetic, pinned against the project's own recorded numbers.
//!
//! These figures are not decorative. `docs/research/mod-tier-data.md` worked out what the map's
//! acceptance-test craft actually costs, and corrected the `0.443%` on
//! [#9](https://github.com/Furizaa/poe-graft/issues/9) — which is a conditional, not a per-click rate.
//! ADR 0002 then removed the roll cap on the strength of the median. If this arithmetic drifts, the
//! window starts telling the human something the research does not say, and nothing else would catch it.

mod common;

use poe_graft_core::Generation;

/// The group behind the map's target: `Flaring` at T1, `Annealed` at T4.
const TARGET: &str = "MinionAddedPhysicalDamage";

/// Compare to a tenth of a basis point — enough to catch a changed model, loose enough to survive
/// floating-point reordering.
fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 5e-7,
        "{what}: got {actual:.8}, expected {expected:.8}"
    );
}

/// `docs/research/mod-tier-data.md`'s table, row by row.
///
/// | ilvl | prefix W | suffix W | conditional | per-click | 1 in | median |
/// | 83   | 38950    | 22100    | 0.4493%     | 0.3680%   | 272  | 188*   |
/// | 84–85| 39125    | 24000    | 0.4473%     | 0.3623%   | 276  | 191    |
/// | ≥ 86 | 39525    | 24000    | 0.4428%     | 0.3591%   | 278  | 193    |
///
/// \* the doc rounds; the exact figure is 188.01 and 188 Rolls is 49.997%, so the smallest count that
/// is actually more likely than not is 189. See `Odds::median_rolls`.
#[test]
fn reproduces_the_recorded_table() {
    let pool = common::pool();

    for (ilvl, conditional, per_click, one_in) in [
        (83u32, 0.004493, 0.003680, 272u32),
        (84, 0.004473, 0.003623, 276),
        (85, 0.004473, 0.003623, 276),
        (86, 0.004428, 0.003591, 278),
        (100, 0.004428, 0.003591, 278),
    ] {
        let odds = pool.odds(TARGET, 1, ilvl).expect("the target group exists");
        assert_close(
            odds.conditional,
            conditional,
            &format!("conditional at ilvl {ilvl}"),
        );
        assert_close(
            odds.per_click,
            per_click,
            &format!("per-click at ilvl {ilvl}"),
        );
        assert_eq!(odds.one_in(), Some(one_in), "1 in N at ilvl {ilvl}");
    }

    let at_83 = pool.odds(TARGET, 1, 83).unwrap();
    assert_eq!(at_83.median_rolls(), Some(189));
    assert_eq!(pool.odds(TARGET, 1, 86).unwrap().median_rolls(), Some(193));
}

/// The pool totals are lower-bound breakpoints, so a level between them keeps the lower row.
#[test]
fn totals_hold_until_the_next_breakpoint() {
    let pool = common::pool();

    let at_83 = pool
        .totals_at(83)
        .expect("83 is above the first breakpoint");
    assert_eq!((at_83.prefix_weight, at_83.suffix_weight), (38950, 22100));

    // 84 and 85 share a row; 86 starts the next one.
    assert_eq!(pool.totals_at(84).unwrap().suffix_weight, 24000);
    assert_eq!(pool.totals_at(85).unwrap().suffix_weight, 24000);
    assert_eq!(pool.totals_at(86).unwrap().prefix_weight, 39525);
}

/// A threshold the jewel cannot support is impossible, not merely unlikely — and the window has to be
/// able to say which. T1 of the target needs item level 83.
#[test]
fn a_tier_the_jewel_cannot_spawn_is_impossible() {
    let pool = common::pool();

    let too_low = pool.odds(TARGET, 1, 82).expect("the group still exists");
    assert!(too_low.is_impossible(), "T1 cannot spawn below ilvl 83");
    assert_eq!(too_low.weight, 0);
    assert_eq!(too_low.one_in(), None);
    assert_eq!(too_low.median_rolls(), None);
    assert_eq!(too_low.cumulative(1_000), 0.0);

    // T2 needs only 72, so the same jewel can still hit a looser threshold.
    assert!(!pool.odds(TARGET, 2, 82).unwrap().is_impossible());
}

/// A looser threshold is strictly more likely, and the weights accumulate down the tiers.
#[test]
fn loosening_the_threshold_only_ever_helps() {
    let pool = common::pool();

    let mut previous = 0.0;
    let mut previous_weight = 0;
    for threshold in 1..=6u8 {
        let odds = pool.odds(TARGET, threshold, 83).unwrap();
        assert!(
            odds.per_click > previous,
            "T{threshold} should beat T{}: {} vs {previous}",
            threshold - 1,
            odds.per_click
        );
        assert!(odds.weight > previous_weight);
        previous = odds.per_click;
        previous_weight = odds.weight;
    }

    // The published per-tier figures for the picker's table.
    for (threshold, one_in) in [(1u8, 272u32), (2, 91), (3, 39), (4, 25), (5, 18), (6, 14)] {
        assert_eq!(
            pool.odds(TARGET, threshold, 83).unwrap().one_in(),
            Some(one_in),
            "1 in N at T{threshold}"
        );
    }
}

/// Cumulative probability is what replaced ADR 0002's roll cap, so it has to behave at the edges.
#[test]
fn cumulative_climbs_and_never_saturates() {
    let pool = common::pool();
    let odds = pool.odds(TARGET, 1, 83).unwrap();

    assert_close(odds.cumulative(0), 0.0, "no rolls, no chance");
    assert_close(odds.cumulative(1), odds.per_click, "one roll is just p");

    // Strictly increasing, and always short of certainty — there is no roll count at which a Hit is
    // guaranteed, which is exactly why the cap was removed.
    let mut previous = 0.0;
    for rolls in [1, 10, 100, 189, 1_000, 10_000] {
        let now = odds.cumulative(rolls);
        assert!(now > previous, "cumulative should climb at {rolls}");
        assert!(now < 1.0, "cumulative should never reach 1 at {rolls}");
        previous = now;
    }

    // The median is the first count that is more likely than not.
    let median = odds.median_rolls().unwrap();
    assert!(odds.cumulative(median) >= 0.5);
    assert!(odds.cumulative(median - 1) < 0.5);
}

/// Suffixes draw against the suffix total, and the two slots differ enough that using the wrong one
/// would be a visible error rather than a rounding one.
#[test]
fn generation_selects_the_right_slot_total() {
    let pool = common::pool();

    let suffix = pool
        .groups()
        .iter()
        .find(|g| g.generation() == Generation::Suffix)
        .expect("this Base has 50 suffix groups");
    let odds = pool.odds(suffix.id(), 1, 83).unwrap();
    let totals = pool.totals_at(83).unwrap();

    let expected_conditional = f64::from(
        suffix
            .tiers()
            .iter()
            .filter(|t| t.tier() == 1 && t.required_ilvl() <= 83)
            .map(|t| t.spawn_weight().expect("the shipped pool carries weights"))
            .sum::<u32>(),
    ) / f64::from(totals.suffix_weight);
    assert_close(odds.conditional, expected_conditional, "suffix conditional");

    // And the prefix total really is the larger of the two at this level, so the slots are not
    // interchangeable.
    assert!(totals.prefix_weight > totals.suffix_weight);
}

/// An unknown group id is `None` rather than a zero — a typo must not read as "impossible".
#[test]
fn an_unknown_group_has_no_odds() {
    let pool = common::pool();
    assert!(pool.odds("NoSuchGroup", 1, 83).is_none());
}

/// A pool with no weights still loads and still assesses Reads; it just has nothing to say about odds.
///
/// This is the whole reason the odds inputs are optional. They are informational, and refusing to start
/// over a number no Verdict consults would make an unrelated feature safety-critical.
#[test]
fn a_pool_without_weights_loads_and_simply_has_no_odds() {
    let json = r##"{
      "base": { "name": "Test Base", "item_class_display": "Tests", "implicits": [] },
      "prefixes": [{
        "group": "Weightless",
        "match_lines": [{ "match_string": "# to something" }],
        "tiers": [{
          "tier": 1, "affix_name": "Untested", "required_ilvl": 1,
          "stats": [{ "display_min": 1.0, "display_max": 2.0 }]
        }]
      }],
      "suffixes": []
    }"##;

    let pool =
        poe_graft_core::ModPool::from_json(json).expect("a pool without weights still parses");
    let group = pool.group_by_id("Weightless").expect("the group is there");
    assert_eq!(group.tiers()[0].spawn_weight(), None);
    assert!(pool.totals_at(83).is_none());
    assert!(
        pool.odds("Weightless", 1, 83).is_none(),
        "no weights means no odds, not impossible odds"
    );

    // And the part that matters still works: the tier still accepts values in band.
    assert!(group.tiers()[0].accepts(&[1.5]));
}

/// Every tier of every group carries a weight, so no group can silently read as impossible.
#[test]
fn every_tier_has_a_spawn_weight() {
    let pool = common::pool();

    for group in pool.groups() {
        for tier in group.tiers() {
            assert!(
                tier.spawn_weight().is_some_and(|w| w > 0),
                "{} T{} has no spawn weight",
                group.id(),
                tier.tier()
            );
        }
        // And every group is reachable at its own worst tier, on a jewel that can hold it.
        let worst = group.tiers().iter().map(|t| t.tier()).max().unwrap();
        let odds = pool.odds(group.id(), worst, 100).unwrap();
        assert!(
            !odds.is_impossible(),
            "{} should be reachable at T{worst}",
            group.id()
        );
    }
}
