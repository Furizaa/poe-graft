# Item Text fixtures

Real **Item Text** captured from Path of Exile, replayed by `cargo test -p poe-graft-core`. This is
what makes ADR 0001's claim about replaying captured fixtures true rather than aspirational; landing
them was [#19](https://github.com/Furizaa/poe-graft/issues/19).

Vocabulary is [`CONTEXT.md`](../../../../CONTEXT.md).

Every file is the clipboard contents verbatim, one capture per file, `\n`-terminated. Nothing has been
reformatted, reordered or corrected — a fixture that looks wrong is evidence, not a typo.

## `captures/spike-17/` — 41 distinct texts from 47 captures

Produced by **our own code** during [the on-device spike](https://github.com/Furizaa/poe-graft/issues/17),
across three armed sessions on 2026-08-04, and recovered from the owner's `poe-graft.log`.

`manifest.json` is the full ordered sequence of all **81 roll records** — every Read the spike
attempted, in order, with the spike's own verdict line verbatim and a pointer to the item text where
there was one. It exists because the parser only needs the 41 distinct texts, but the roll *cycle*
([#20](https://github.com/Furizaa/poe-graft/issues/20)) needs the sequence: 47 records carry text, 34
do not, and 5 of the 47 are the spike's `IDENTICAL to the previous roll` case, which is exactly the
stale-Read condition ADR 0002 turns into an `Unknown` Verdict.

The 34 textless records are not noise either — they are what a settle delay of 40 ms does. All three
sessions ran at 40 ms, which [#17](https://github.com/Furizaa/poe-graft/issues/17) later found to be
below the game's item-update floor; the shipped default is 130 ms. Treat the failure *rate* in this
manifest as a measurement of a delay we no longer use.

Only roll records were taken from the log. The surrounding lines — screen geometry, cursor
coordinates, window titles, the owner's typed notes — describe the owner's machine rather than the
game, and this repository is public.

### What this set covers

- **Both affix slots and both counts.** Prefix-only, suffix-only, and one-of-each items.
- **33 of the base's 66 mod groups**, at every tier from 1 to 6.
- **The target group at two tiers** — `05-annealed-of-order.txt` is T4 `Annealed`, and
  `15-razor-sharp.txt` is T3 `Razor-sharp`. Both are `MinionAddedPhysicalDamage`, which is the trap
  #19 was told to watch for: nothing may match a single affix name.
- **An annotation with no tag list** — `13-resonating-of-instinct.txt` carries
  `{ Suffix Modifier "of Instinct" (Tier: 1) }`, no em-dash and no tags.
- **Decimal rolled values against integer bounds** — `Regenerate 11.6(9-12) Life per second`, and
  decimal bounds too: `Regenerate 3.6(3.3-4) Mana per second`.
- **Descriptive continuation lines inside the explicit-mod section** that are not mods and do contain
  numbers, e.g. `(Being Blinded causes 20% less Accuracy Rating and Evasion Rating, for 4 seconds)`.
- **Affix names that do not survive naive slugging** — `Razor-sharp`, `of the Hearth`.

### What it does not cover

- **Advanced Mod Descriptions off.** Every one of the 47 carries the annotation and inline bounds, so
  the plain display form has *no* real capture behind it. The tests derive it from these files by
  stripping annotations and inline bounds, which is what the setting is believed to do; that belief is
  untested on device.
- **Anything but `Rarity: Magic`, `Item Level: 83`, Ghastly Eye Jewel.** One base, one item level, one
  rarity — a Craft Session never sees anything else, but the wrong-item `Halt` has only synthetic
  fixtures behind it.
- **Corrupted items, implicits, enchantments.** The base has no implicits and the spike never rolled a
  corrupted jewel, so section awareness is exercised against synthetic text only.

## `captures/ahk-16/` — one distinct text from 113 captures

From the AutoHotkey-era probe in [a comment on #16](https://github.com/Furizaa/poe-graft/issues/16),
kept because it is independent evidence of the format from a different tool.

**Its diversity is one.** 115 records: 113 successes and 2 empty timeouts, and all 113 successes are
byte-identical — the probe copied the same jewel repeatedly without ever rolling it, which #16's own
analysis says outright. As parser input this set is worth exactly one file, so one file is what is
here; `manifest.json` records all 115 records so the count stays honest.

Its value is corroboration, and it delivers: captured by a different tool, months earlier, it agrees
with `spike-17/` on section layout, on the annotation format, and on inline bounds. It also
cross-checks the tier data — `Glaciated` (Tier: 3) with `23(20-23) to 28(26-32)` matches
`MinionAddedColdDamage` tier 3 in `data/ghastly-eye-jewel.json` exactly, name and both ranges.

## Adding to these sets

Land captures verbatim, under a directory named for the ticket that produced them, with a manifest if
the order or the failures matter. Do not hand-write a file into `captures/` — synthetic text belongs
in the tests that build it, where it is obvious that no one ever saw it come out of the game.
