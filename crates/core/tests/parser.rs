//! The Item Text parser.

mod common;

use poe_graft_core::{parse_item_text, Generation, ModPool, Rarity, Unreadable};

#[test]
fn parses_the_identity_and_the_explicit_mods_of_a_real_capture() {
    let pool = common::pool();
    let text = common::capture("spike-17/05-annealed-of-order.txt");

    let item = parse_item_text(&text, &pool).expect("a real capture parses");

    // What a Craft Session pins. `Requirements: Level:` is deliberately not part of it — it moves
    // with the mods that rolled, so pinning it would make every Roll look like a different item.
    assert_eq!(item.identity().rarity(), Rarity::Magic);
    assert_eq!(item.identity().base_name(), "Ghastly Eye Jewel");
    assert_eq!(item.identity().item_level(), 83);

    let mods = item.mods();
    assert_eq!(mods.len(), 2, "one prefix and one suffix");

    assert_eq!(mods[0].group_id(), "MinionAddedPhysicalDamage");
    assert_eq!(mods[0].generation(), Generation::Prefix);
    assert_eq!(
        mods[0].match_strings(),
        ["Minions deal # to # additional Physical Damage"]
    );
    assert_eq!(mods[0].values(), [12.0, 18.0]);

    // The `+` belongs to the value placeholder, so it is consumed with the number rather than being
    // part of the group's rendered line.
    assert_eq!(mods[1].group_id(), "ChaosResistanceForJewel");
    assert_eq!(mods[1].generation(), Generation::Suffix);
    assert_eq!(mods[1].match_strings(), ["#% to Chaos Resistance"]);
    assert_eq!(mods[1].values(), [8.0]);

    assert!(item.base_matches_pool());
    assert!(item.unrecognised().is_empty());
    assert!(item.annotated());

    // The annotation is read and carried, so it can be logged and cross-checked. It never reaches a
    // Verdict — that is the hit test's business, and `hit_test.rs` is where that is asserted.
    let annotation = mods[0].annotation().expect("this capture is annotated");
    assert_eq!(annotation.generation(), Generation::Prefix);
    assert_eq!(annotation.affix_name(), "Annealed");
    assert_eq!(annotation.tier(), 4);
    assert_eq!(annotation.tags(), ["Damage", "Physical", "Minion"]);
}

#[test]
fn a_mod_that_renders_two_lines_is_one_mod_with_both_values() {
    let pool = common::pool();
    // `of Training` prints an Attack Speed line and a Cast Speed line. Reading them as two mods
    // would compare one value against a tier that has two bands, and match nothing.
    let text = common::capture("spike-17/09-fuelling-of-training.txt");

    let item = parse_item_text(&text, &pool).expect("a real capture parses");

    assert_eq!(item.mods().len(), 2, "one prefix and one suffix");
    let training = item
        .mod_of_group("MinionAttackAndCastSpeed")
        .expect("the suffix is the two-line group");
    assert_eq!(
        training.match_strings(),
        [
            "Minions have #% increased Attack Speed",
            "Minions have #% increased Cast Speed",
        ]
    );
    assert_eq!(training.values(), [5.0, 4.0]);
    assert!(item.unrecognised().is_empty());
}

#[test]
fn an_item_with_no_requirements_section_still_parses() {
    let pool = common::pool();
    // This capture has no `Requirements:` section at all — the mods that rolled impose no level
    // requirement. Anything locating the explicit-mod section by counting sections breaks here.
    let text = common::capture("spike-17/09-fuelling-of-training.txt");
    assert!(
        !text.contains("Requirements:"),
        "the fixture this test is about has changed"
    );

    let item = parse_item_text(&text, &pool).expect("a real capture parses");

    assert_eq!(item.identity().item_level(), 83);
    assert_eq!(item.mods().len(), 2);
}

#[test]
fn descriptions_and_the_trailer_are_not_mods() {
    let pool = common::pool();
    // Carries `(Recently refers to the past 4 seconds)` inside the explicit-mod section, and the
    // `Place into an Abyssal Socket…` trailer after it. Neither is a mod, and the description even
    // contains a number, so it cannot be excluded by looking for digits.
    let text = common::capture("spike-17/01-fuelling-of-retaliation.txt");

    let item = parse_item_text(&text, &pool).expect("a real capture parses");

    assert_eq!(item.mods().len(), 2, "the description is not a third mod");
    assert!(
        item.unrecognised().is_empty(),
        "unrecognised: {:?}",
        item.unrecognised()
    );
    // `Minions Regenerate # Life per second` is its own group, distinct from the non-minion
    // `Regenerate # Life per second` — the two differ by one word and by six tiers.
    assert_eq!(item.mods()[0].group_id(), "FlatMinionLifeRegeneration");
    assert_eq!(item.mods()[1].group_id(), "CastSpeedIfMinionKilledRecently");
}

#[test]
fn a_decimal_roll_against_integer_bounds_parses() {
    let pool = common::pool();
    // `Regenerate 11.6(9-12) Life per second` — the game prints a decimal roll inside integer
    // bounds. Reading values as integers would lose the fraction and could shift the derived tier.
    let (name, text) = common::capture_set("spike-17")
        .into_iter()
        .find(|(_, t)| t.contains("Regenerate 11.6"))
        .expect("a capture with a decimal roll");

    let item = parse_item_text(&text, &pool).unwrap_or_else(|e| panic!("{name}: {e}"));

    let regen = item
        .mod_of_group("LifeRegeneration")
        .expect("the decimal mod");
    assert_eq!(regen.values(), [11.6]);
}

#[test]
fn every_real_capture_parses_with_nothing_unrecognised() {
    let pool = common::pool();

    for (name, text) in common::capture_set("spike-17")
        .into_iter()
        .chain(common::capture_set("ahk-16"))
    {
        let item = parse_item_text(&text, &pool).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert!(
            item.unrecognised().is_empty(),
            "{name}: the pool did not recognise {:?}",
            item.unrecognised()
        );
        assert!(item.base_matches_pool(), "{name}: not the pool's base");
        assert_eq!(item.identity().rarity(), Rarity::Magic, "{name}");
        assert_eq!(item.identity().item_level(), 83, "{name}");
        assert!(
            (1..=2).contains(&item.mods().len()),
            "{name}: a Magic item has one or two mods, found {}",
            item.mods().len()
        );
        // Every capture we have was taken with Advanced Mod Descriptions on, and every mod on it
        // therefore carries an annotation. `both_display_forms_agree` covers the other case.
        assert!(item.annotated(), "{name}");
        for m in item.mods() {
            assert!(
                m.annotation().is_some(),
                "{name}: {:?} lost its annotation",
                m.group_id()
            );
        }
    }
}

#[test]
fn both_display_forms_agree_on_every_real_capture() {
    let pool = common::pool();

    for (name, annotated) in common::capture_set("spike-17")
        .into_iter()
        .chain(common::capture_set("ahk-16"))
    {
        let plain = common::without_advanced_descriptions(&annotated);
        assert!(
            !plain.contains('{'),
            "{name}: the stripper left annotations"
        );
        assert!(
            !plain.contains("(9-12)") && !plain.contains("(20-23)"),
            "{name}: the stripper left inline bounds"
        );

        let from_annotated =
            parse_item_text(&annotated, &pool).unwrap_or_else(|e| panic!("{name}: {e}"));
        let from_plain =
            parse_item_text(&plain, &pool).unwrap_or_else(|e| panic!("{name} (plain): {e}"));

        assert_eq!(
            from_annotated.identity(),
            from_plain.identity(),
            "{name}: identity differs between display forms"
        );
        assert!(!from_plain.annotated(), "{name}");

        // The mods must agree on everything the Verdict is computed from. Only the annotation
        // differs — which is the entire point of deriving tier from the numbers.
        let strip = |item: &poe_graft_core::Item| -> Vec<(String, Vec<f64>, Vec<String>)> {
            item.mods()
                .iter()
                .map(|m| {
                    (
                        m.group_id().to_string(),
                        m.values().to_vec(),
                        m.match_strings().to_vec(),
                    )
                })
                .collect()
        };
        assert_eq!(
            strip(&from_annotated),
            strip(&from_plain),
            "{name}: mods differ between display forms"
        );
    }
}

#[test]
fn every_capture_of_the_same_jewel_has_the_same_identity() {
    let pool = common::pool();
    // The 41 spike captures are one physical jewel, rerolled. The identity a Craft Session pins must
    // therefore be equal across all of them, or #20's wrong-item Halt would fire on every Roll.
    //
    // This is what excludes `Requirements: Level:` from the identity: it moves with the mods, and
    // one capture has no `Requirements:` section at all.
    let captures = common::capture_set("spike-17");

    let mut requirement_levels = std::collections::BTreeSet::new();
    for (_, text) in &captures {
        let level = text
            .lines()
            .find_map(|l| l.strip_prefix("Level: "))
            .map(str::to_string);
        requirement_levels.insert(level);
    }
    assert!(
        requirement_levels.len() > 1,
        "these captures are supposed to disagree about the requirement level, found \
         {requirement_levels:?} — if they now agree, this test proves nothing"
    );

    let mut identities = captures.into_iter().map(|(name, text)| {
        let item = parse_item_text(&text, &pool).unwrap_or_else(|e| panic!("{name}: {e}"));
        (name, item.identity().clone())
    });
    let (first_name, first) = identities.next().expect("there are captures");
    for (name, identity) in identities {
        assert_eq!(identity, first, "{name} differs from {first_name}");
    }
}

#[test]
fn a_different_item_level_is_a_different_identity() {
    let pool = common::pool();
    // ilvl 82 cannot roll T1 `Flaring` at all, so mistaking one jewel for the other would silently
    // make the Target Mod unreachable.
    let text = common::capture("spike-17/05-annealed-of-order.txt");
    let lower = text.replace("Item Level: 83", "Item Level: 82");

    let a = parse_item_text(&text, &pool).expect("parses");
    let b = parse_item_text(&lower, &pool).expect("parses");

    assert_eq!(b.identity().item_level(), 82);
    assert_ne!(a.identity(), b.identity());
}

#[test]
fn a_rarity_the_app_does_not_know_is_unreadable() {
    let pool = common::pool();
    // Fail closed: an unreadable rarity is an `Unknown` Verdict, which never spends an orb.
    let text = common::capture("spike-17/05-annealed-of-order.txt")
        .replace("Rarity: Magic", "Rarity: Wondrous");

    let err = parse_item_text(&text, &pool).expect_err("an unknown rarity is not readable");

    assert!(matches!(err, Unreadable::Rarity(_)), "{err:?}");
    assert!(err.to_string().contains("Wondrous"), "{err}");
}

#[test]
fn text_that_is_not_an_item_is_unreadable() {
    let pool = common::pool();
    // What the clipboard holds when the game never copied and something else was there instead.
    let err = parse_item_text("https://example.invalid/some-link", &pool)
        .expect_err("arbitrary text is not an item");

    assert_eq!(err, Unreadable::NotItemText);
}

#[test]
fn a_missing_item_level_is_unreadable() {
    let pool = common::pool();
    // The explicit-mod section is located relative to `Item Level:`, so without it the parser has no
    // idea which section holds the mods. Refusing is the only safe answer.
    let text = common::capture("spike-17/05-annealed-of-order.txt").replace("Item Level: 83\n", "");

    let err = parse_item_text(&text, &pool).expect_err("no item level, no section anchor");

    assert!(matches!(err, Unreadable::ItemLevel(_)), "{err:?}");
}

#[test]
fn a_base_with_implicits_is_refused_rather_than_guessed_at() {
    // Ghastly Eye Jewel has none. An implicit section would sit between `Item Level:` and the
    // explicit mods and shift the section the parser reads — so the day a Base with implicits is
    // added, the parser must refuse rather than assess the wrong section.
    let pool = ModPool::from_json(POOL_WITH_IMPLICITS).expect("the synthetic pool parses");
    assert_eq!(pool.implicit_count(), 1);

    let err = parse_item_text("Item Class: Abyss Jewels\nRarity: Magic\nX\n", &pool)
        .expect_err("a base with implicits is not supported");

    assert_eq!(err, Unreadable::ImplicitsUnsupported { count: 1 });
}

const POOL_WITH_IMPLICITS: &str = r#"{
  "base": {
    "name": "Ghastly Eye Jewel",
    "item_class_display": "Abyss Jewels",
    "implicits": ["SomeImplicitMod"]
  },
  "prefixes": [],
  "suffixes": []
}"#;
