//! The hit test: a parsed item plus a Target Mod becomes a Verdict.

mod common;

use poe_graft_core::{assess, parse_item_text, Diagnostic, Target, Verdict};

const TARGET: &str = "MinionAddedPhysicalDamage";

#[test]
fn the_t4_capture_is_a_miss_at_threshold_1_and_a_hit_at_threshold_4() {
    let pool = common::pool();
    // `Minions deal 12(9-12) to 18(15-18) additional Physical Damage` — T4 `Annealed`. The
    // acceptance case from issue #19, and the whole point of deriving tier from the numbers: 12 and
    // 18 land inside T4's bands and nowhere else.
    let item = parse_item_text(&common::capture("spike-17/05-annealed-of-order.txt"), &pool)
        .expect("a real capture parses");

    let strict = assess(&item, &Target::new(TARGET, 1), &pool);
    assert_eq!(strict.verdict(), Verdict::Miss);
    assert_eq!(strict.tier(), Some(4));

    let relaxed = assess(&item, &Target::new(TARGET, 4), &pool);
    assert_eq!(relaxed.verdict(), Verdict::Hit);
    assert_eq!(relaxed.tier(), Some(4));
}

#[test]
fn the_same_group_at_a_different_tier_has_a_different_affix_name() {
    let pool = common::pool();
    // `Razor-sharp` is T3 of the *same* group `Annealed` is T4 of. Anything keyed on the affix name
    // would treat these as unrelated mods — the trap issue #4 walked into.
    let item = parse_item_text(&common::capture("spike-17/15-razor-sharp.txt"), &pool)
        .expect("a real capture parses");

    assert!(item.mod_of_group(TARGET).is_some());
    assert_eq!(
        item.mod_of_group(TARGET)
            .and_then(|m| m.annotation())
            .map(|a| a.affix_name()),
        Some("Razor-sharp")
    );

    assert_eq!(
        assess(&item, &Target::new(TARGET, 3), &pool).tier(),
        Some(3)
    );
    assert_eq!(
        assess(&item, &Target::new(TARGET, 3), &pool).verdict(),
        Verdict::Hit
    );
    assert_eq!(
        assess(&item, &Target::new(TARGET, 2), &pool).verdict(),
        Verdict::Miss
    );
}

#[test]
fn an_item_without_the_target_group_is_a_miss() {
    let pool = common::pool();
    // Cold damage, not physical. A Miss must be positive — it is the only Verdict that spends an orb.
    let item = parse_item_text(&common::capture("spike-17/28-glaciated.txt"), &pool)
        .expect("a real capture parses");

    let assessment = assess(&item, &Target::new(TARGET, 1), &pool);

    assert_eq!(assessment.verdict(), Verdict::Miss);
    assert_eq!(assessment.tier(), None);
    assert!(assessment.diagnostics().is_empty());
    assert!(!assessment.halt_worthy());
}

#[test]
fn the_derived_tier_agrees_with_the_game_on_every_real_capture() {
    let pool = common::pool();
    // The strongest check in the suite. For every mod of every capture, the tier our numbers imply
    // is the tier the game itself annotated — across 33 Mod Groups and tiers 1 to 6. If the tier
    // data has a wrong row, this is what finds it.
    let mut checked = 0;

    for (name, text) in common::capture_set("spike-17")
        .into_iter()
        .chain(common::capture_set("ahk-16"))
    {
        let item = parse_item_text(&text, &pool).unwrap_or_else(|e| panic!("{name}: {e}"));

        for m in item.mods() {
            let annotated = m.annotation().expect("captures are annotated").tier();
            let group = pool.group_by_id(m.group_id()).expect("a parsed group");
            let matching: Vec<u8> = group
                .tiers()
                .iter()
                .filter(|t| t.accepts(m.values()))
                .map(|t| t.tier())
                .collect();

            assert_eq!(
                matching,
                vec![annotated],
                "{name}: {:?} rolled {:?}, which the game calls tier {annotated}",
                m.lines(),
                m.values(),
            );
            checked += 1;
        }
    }

    // 64 mods across the 42 captures as landed. A guard, so that a fixture directory that quietly
    // empties out cannot make this test pass by checking nothing.
    assert!(checked >= 64, "only {checked} mods cross-checked");
}

#[test]
fn both_display_forms_reach_the_same_verdict_on_every_real_capture() {
    let pool = common::pool();
    // The reason tier is derived from numbers rather than read off the annotation: turning Advanced
    // Mod Descriptions off must change nothing about safety.
    for (name, annotated) in common::capture_set("spike-17") {
        let plain = common::without_advanced_descriptions(&annotated);

        for threshold in 1..=6 {
            let target = Target::new(TARGET, threshold);
            let a = assess(
                &parse_item_text(&annotated, &pool).unwrap_or_else(|e| panic!("{name}: {e}")),
                &target,
                &pool,
            );
            let p = assess(
                &parse_item_text(&plain, &pool).unwrap_or_else(|e| panic!("{name} plain: {e}")),
                &target,
                &pool,
            );

            assert_eq!(a.verdict(), p.verdict(), "{name} at T{threshold}");
            assert_eq!(a.tier(), p.tier(), "{name} at T{threshold}");
        }
    }
}

#[test]
fn a_missing_annotation_is_reported_and_changes_no_verdict() {
    let pool = common::pool();
    let annotated = common::capture("spike-17/05-annealed-of-order.txt");
    let plain = common::without_advanced_descriptions(&annotated);

    let item = parse_item_text(&plain, &pool).expect("the plain form parses");
    let assessment = assess(&item, &Target::new(TARGET, 4), &pool);

    assert_eq!(assessment.verdict(), Verdict::Hit);
    assert_eq!(assessment.tier(), Some(4));
    assert!(matches!(
        assessment.diagnostics(),
        [Diagnostic::AnnotationAbsent { .. }]
    ));
    // Expected on a client with the setting off. Not a reason to stop crafting.
    assert!(!assessment.halt_worthy());
}

#[test]
fn an_annotation_that_disagrees_is_a_halt_and_still_does_not_move_the_verdict() {
    let pool = common::pool();
    // The game says T1 while the numbers say T4. That means our tier data is wrong, which ADR 0002
    // makes a Halt — but the Verdict still comes from the numbers, not from the annotation.
    let text = common::capture("spike-17/05-annealed-of-order.txt")
        .replace("(Tier: 4)", "(Tier: 1)")
        .replace("\"Annealed\"", "\"Flaring\"");

    let item = parse_item_text(&text, &pool).expect("still parses");
    let assessment = assess(&item, &Target::new(TARGET, 1), &pool);

    assert_eq!(
        assessment.verdict(),
        Verdict::Miss,
        "the annotation claimed T1, which would have been a Hit"
    );
    assert_eq!(assessment.tier(), Some(4));
    assert!(matches!(
        assessment.diagnostics(),
        [Diagnostic::AnnotationDisagrees {
            derived: 4,
            annotated: 1,
            ..
        }]
    ));
    assert!(assessment.halt_worthy());
}

#[test]
fn values_matching_no_tier_fail_closed_to_a_hit() {
    let pool = common::pool();
    // Deliberately impossible numbers. Ambiguity resolves to a Hit: the safe direction is to stop.
    let text = common::capture("spike-17/05-annealed-of-order.txt").replace(
        "Minions deal 12(9-12) to 18(15-18) additional Physical Damage",
        "Minions deal 999 to 999 additional Physical Damage",
    );

    let item = parse_item_text(&text, &pool).expect("still parses");
    // Threshold 6 is the most permissive there is, so a Miss here could not be blamed on the tier.
    let assessment = assess(&item, &Target::new(TARGET, 6), &pool);

    assert_eq!(assessment.verdict(), Verdict::Hit);
    assert_eq!(assessment.tier(), None, "no tier was derived");
    assert!(matches!(
        assessment.diagnostics(),
        [Diagnostic::NoTierMatched { .. }]
    ));
}

#[test]
fn overlapping_tiers_fail_closed_to_a_hit() {
    // The shipped pool has no overlapping tiers, so this needs a synthetic one. It is a test rather
    // than a comment because the fail-closed branch has to be known to work before the day a data
    // change makes it reachable.
    let pool = poe_graft_core::ModPool::from_json(OVERLAPPING_POOL).expect("the synthetic pool");
    let item = parse_item_text(SYNTHETIC_ITEM, &pool).expect("the synthetic item parses");

    let assessment = assess(&item, &Target::new("Overlapping", 1), &pool);

    assert_eq!(assessment.verdict(), Verdict::Hit);
    assert_eq!(assessment.tier(), None);
    assert!(
        matches!(
            assessment.diagnostics(),
            [Diagnostic::ManyTiersMatched { tiers, .. }] if tiers == &[1, 2]
        ),
        "{:?}",
        assessment.diagnostics()
    );
}

#[test]
fn the_shipped_pool_has_no_overlapping_tiers() {
    let pool = common::pool();
    // Issue #4 established this, and the whole numeric derivation rests on it. Asserted here so the
    // day it stops being true is the day CI says so, rather than the day a craft goes wrong.
    for group in pool.groups() {
        let tiers = group.tiers();
        for (i, a) in tiers.iter().enumerate() {
            for b in &tiers[i + 1..] {
                let overlaps = a.bands().len() == b.bands().len()
                    && a.bands()
                        .iter()
                        .zip(b.bands())
                        .all(|(x, y)| x.min() <= y.max() && y.min() <= x.max());
                assert!(
                    !overlaps,
                    "{}: T{} and T{} overlap",
                    group.id(),
                    a.tier(),
                    b.tier()
                );
            }
        }
    }
}

#[test]
fn a_target_mod_outside_the_explicit_section_is_not_a_hit() {
    let pool = common::pool();
    // A perfect T1 roll of the target group, sitting where an implicit and an enchantment would be.
    // Without section awareness either would read as a Hit — and ADR 0002 refuses to lean on the
    // `Prefix Modifier` annotation to tell them apart, so the section boundaries have to carry it.
    let t1_line = "Minions deal 26(23-26) to 39(33-39) additional Physical Damage";
    let text = format!(
        "Item Class: Abyss Jewels\n\
         Rarity: Magic\n\
         Glaciated Ghastly Eye Jewel\n\
         --------\n\
         Abyss\n\
         --------\n\
         {{ Implicit Modifier \"Flaring\" (Tier: 1) — Damage, Physical, Minion }}\n\
         {t1_line}\n\
         --------\n\
         Item Level: 83\n\
         --------\n\
         {{ Prefix Modifier \"Glaciated\" (Tier: 3) — Damage, Elemental, Cold, Minion }}\n\
         Minions deal 23(20-23) to 28(26-32) additional Cold Damage\n\
         --------\n\
         {{ Enchant Modifier \"Flaring\" (Tier: 1) — Damage, Physical, Minion }}\n\
         {t1_line}\n\
         --------\n\
         Place into an Abyssal Socket on an Item or into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.\n"
    );

    let item = parse_item_text(&text, &pool).expect("parses");

    assert_eq!(item.mods().len(), 1, "only the explicit section counts");
    assert_eq!(item.mods()[0].group_id(), "MinionAddedColdDamage");

    let assessment = assess(&item, &Target::new(TARGET, 1), &pool);
    assert_eq!(assessment.verdict(), Verdict::Miss);
    assert!(!assessment.halt_worthy());
}

#[test]
fn a_different_base_establishes_nothing_rather_than_reading_as_a_miss() {
    let pool = common::pool();
    // A Cobalt Jewel is not a Ghastly Eye Jewel. Calling this a Miss would let the app spend an orb
    // on an item it was never asked to craft.
    let text = common::capture("spike-17/28-glaciated.txt")
        .replace("Glaciated Ghastly Eye Jewel", "Glaciated Cobalt Jewel");

    let item = parse_item_text(&text, &pool).expect("parses — the cycle needs to see what it is");
    assert!(!item.base_matches_pool());

    let assessment = assess(&item, &Target::new(TARGET, 1), &pool);
    assert_eq!(assessment.verdict(), Verdict::Unknown);
    assert!(matches!(
        assessment.diagnostics(),
        [Diagnostic::NotTheBase { .. }]
    ));
    // A wrong item is a Halt rather than a Resync, or the app would re-read it forever.
    assert!(assessment.halt_worthy());
}

#[test]
fn an_unrecognised_mod_line_establishes_nothing() {
    let pool = common::pool();
    // The game's wording changed, or the pool is missing a mod. Either way the app is looking at an
    // item it does not understand, and a Miss would spend an orb on that basis.
    let original = common::capture("spike-17/28-glaciated.txt");
    let text = original.replace("additional Cold Damage", "supplementary Cold Damage");
    assert_ne!(text, original, "the mutation this test needs did not apply");

    let item = parse_item_text(&text, &pool).expect("parses");
    assert_eq!(item.unrecognised().len(), 1);

    let assessment = assess(&item, &Target::new(TARGET, 1), &pool);
    assert_eq!(assessment.verdict(), Verdict::Unknown);
    assert!(assessment.halt_worthy());
}

#[test]
fn a_target_the_pool_does_not_have_establishes_nothing() {
    let pool = common::pool();
    let item = parse_item_text(&common::capture("spike-17/28-glaciated.txt"), &pool)
        .expect("a real capture parses");

    let assessment = assess(&item, &Target::new("NoSuchGroup", 1), &pool);

    assert_eq!(assessment.verdict(), Verdict::Unknown);
    assert!(matches!(
        assessment.diagnostics(),
        [Diagnostic::UnknownTarget { .. }]
    ));
    assert!(assessment.halt_worthy());
}

/// A pool whose tier 1 and tier 2 bands overlap, which the shipped data never does.
const OVERLAPPING_POOL: &str = r#"{
  "base": {
    "name": "Ghastly Eye Jewel",
    "item_class_display": "Abyss Jewels",
    "implicits": []
  },
  "prefixes": [
    {
      "group": "Overlapping",
      "match_lines": [{ "match_string": "Minions deal # to # additional Physical Damage" }],
      "tiers": [
        {
          "tier": 1,
          "affix_name": "Flaring",
          "required_ilvl": 83,
          "stats": [
            { "display_min": 10, "display_max": 20 },
            { "display_min": 10, "display_max": 20 }
          ]
        },
        {
          "tier": 2,
          "affix_name": "Tempered",
          "required_ilvl": 72,
          "stats": [
            { "display_min": 15, "display_max": 25 },
            { "display_min": 15, "display_max": 25 }
          ]
        }
      ]
    }
  ],
  "suffixes": []
}"#;

/// Values inside both tiers of [`OVERLAPPING_POOL`].
const SYNTHETIC_ITEM: &str = "Item Class: Abyss Jewels\n\
    Rarity: Magic\n\
    Flaring Ghastly Eye Jewel\n\
    --------\n\
    Item Level: 83\n\
    --------\n\
    Minions deal 18 to 18 additional Physical Damage\n";
