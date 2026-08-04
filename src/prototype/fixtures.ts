/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * Real Item Text, copied out of `crates/core/tests/fixtures/captures/spike-17/`.
 *
 * Copied rather than imported because the fixtures README forbids hand-writing files *into*
 * `captures/`, not reading them out — and a variant that renders the last Read has to render
 * something with real density, not a tidied-up invention. Three were chosen to cover the shapes that
 * break a naive layout:
 *
 * - **05** — the target group at T4 plus a suffix. Two mods, the ordinary case.
 * - **28** — a single prefix and nothing else. A Miss, and the shortest text there is.
 * - **09** — one mod rendering **two** stat lines, and **no `Requirements:` section**. Any variant
 *   that assumes one line per mod, or a fixed set of sections, is wrong here.
 */

export type Capture = {
  name: string;
  /** What it is worth looking at. Prototype chrome only — not variant copy. */
  note: string;
  text: string;
};

export const captures: Capture[] = [
  {
    name: "05-annealed-of-order",
    note: "target group at T4, prefix + suffix",
    text: `Item Class: Abyss Jewels
Rarity: Magic
Annealed Ghastly Eye Jewel of Order
--------
Abyss
--------
Requirements:
Level: 43
--------
Item Level: 83
--------
{ Prefix Modifier "Annealed" (Tier: 4) — Damage, Physical, Minion }
Minions deal 12(9-12) to 18(15-18) additional Physical Damage
{ Suffix Modifier "of Order" (Tier: 1) — Chaos, Resistance }
+8(7-13)% to Chaos Resistance
--------
Place into an Abyssal Socket on an Item or into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.`,
  },
  {
    name: "28-glaciated",
    note: "a Miss — one prefix, nothing else",
    text: `Item Class: Abyss Jewels
Rarity: Magic
Glaciated Ghastly Eye Jewel
--------
Abyss
--------
Requirements:
Level: 46
--------
Item Level: 83
--------
{ Prefix Modifier "Glaciated" (Tier: 3) — Damage, Elemental, Cold, Minion }
Minions deal 20(20-23) to 26(26-32) additional Cold Damage
--------
Place into an Abyssal Socket on an Item or into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.`,
  },
  {
    name: "09-fuelling-of-training",
    note: "one mod, two stat lines, and no Requirements section",
    text: `Item Class: Abyss Jewels
Rarity: Magic
Fuelling Ghastly Eye Jewel of Training
--------
Abyss
--------
Item Level: 83
--------
{ Prefix Modifier "Fuelling" (Tier: 3) — Life, Minion }
Minions Regenerate 29(22-30) Life per second
{ Suffix Modifier "of Training" (Tier: 1) — Attack, Caster, Speed, Minion }
Minions have 5(4-6)% increased Attack Speed
Minions have 4(4-6)% increased Cast Speed
--------
Place into an Abyssal Socket on an Item or into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.`,
  },
];

/** Just the explicit-mod section — the only section a Verdict is ever allowed to consult. */
export const explicitMods = (text: string): string[] => {
  const sections = text.split("\n--------\n");
  const found = sections.find((s) => s.trimStart().startsWith("{"));
  return found ? found.trim().split("\n") : [];
};

/** `Item Level: 83` → 83. The parser already knows this mid-session; the picker does not. */
export const itemLevel = (text: string): number | null => {
  const match = /^Item Level: (\d+)$/m.exec(text);
  return match ? Number(match[1]) : null;
};
