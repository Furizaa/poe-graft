//! The mod pool: `data/ghastly-eye-jewel.json` as the core sees it.

mod common;

#[test]
fn the_shipped_pool_knows_the_target_group_by_its_rendered_line() {
    let pool = common::pool();

    // The line as the game renders it, with the rolled values replaced by `#` — the only handle the
    // parser has, because a Mod Group's display name is per tier and so identifies nothing.
    let group = pool
        .group("Minions deal # to # additional Physical Damage")
        .expect("the target group is in the shipped pool");

    assert_eq!(group.id(), "MinionAddedPhysicalDamage");
    assert_eq!(group.tier_count(), 6);

    // ADR 0002 and issue #4: the same group, two names. Anything matching a single name is a bug.
    assert_eq!(group.tier(1).expect("tier 1").affix_name(), "Flaring");
    assert_eq!(group.tier(4).expect("tier 4").affix_name(), "Annealed");
}
