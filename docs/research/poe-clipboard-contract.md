# PoE 1 hover-copy clipboard contract and timing

Research for [issue #3](https://github.com/Furizaa/poe-graft/issues/3). Written 2026-08-04 against
Path of Exile **3.29.x** (Curse of the Allflame).

Everything below is sourced from the source code of shipping tools, Microsoft's Win32 documentation,
and GGG patch notes. Claims that could only be settled by running the game are collected in
[Open questions requiring the game](#open-questions-requiring-the-game) and are **not** asserted
anywhere else.

---

## Verdict / recommended read protocol

The clipboard hover-copy is a sound primary read mechanism for poe-graft, and the copied text is
richer than the map assumed: **it already contains prefix/suffix, the affix name, the tier number,
and the tier's numeric range**. No tier derivation from the roll is needed.

The staleness problem is solvable, and the solution is not content comparison. It is:

> **Poison the clipboard with a value the game can never produce, then require that the value is
> gone.**

That converts "is this text different from the previous roll?" (unanswerable — two alteration rolls
can be byte-identical) into "did anything at all replace my sentinel?" (trivially answerable). The
sequence number is a second, independent witness that also tells you *which* failure occurred.

### The protocol

```
read_hovered_item() -> Fresh(text) | Stale | Timeout
──────────────────────────────────────────────────────────────────────
 0. PRECONDITION  assert the PoE window is foreground. If it is not, either
                  focus it and wait for the activation to land, or fail the
                  read. An unfocused client does not copy. (Exile-UI, APT)

 1. POISON        Write a per-read unique sentinel to the clipboard:
                      __POE_GRAFT_<monotonic_counter>_<timestamp_ns>
                  A sentinel, not an empty string:
                    - empty-clipboard writes are silently undone by some
                      clipboard managers (KDE "Prevent empty clipboard";
                      APT hit this and switched to a sentinel on Linux),
                    - a sentinel is self-identifying in logs, which matters
                      because there is no dev environment on the Windows box.
                  Open/Close the clipboard around this write and hold it open
                  for as short a time as possible — see step 2's hazard.

 2. SNAPSHOT      seq0 = GetClipboardSequenceNumber()      (AFTER the poison
                  write, so the poison's own increment is already counted)

                  HAZARD: OpenClipboard "fails if another window has the
                  clipboard open". If poe-graft still holds the clipboard when
                  the game tries to copy, the game's copy fails silently and
                  the clipboard keeps the sentinel. This is exactly why the
                  sentinel matters — that failure is then *detected*, not
                  mistaken for a roll.

 3. SEND          One synthesized Ctrl+C via SendInput:
                    a. read physical modifier state (GetAsyncKeyState) and
                       release any modifier we did not intend — SendInput
                       "does not reset the keyboard's current state",
                    b. Ctrl down, C down, C up, Ctrl up,
                    c. do NOT hold Alt. Since 3.29 Ctrl+C alone yields the
                       advanced description format, and holding the advanced-
                       descriptions modifier now pins in-game tooltips
                       (APT issue #1865).

 4. POLL          Poll every ~10-15 ms up to a 400 ms deadline. On each tick:
                    changed = GetClipboardSequenceNumber() != seq0
                    if !changed:  continue      // nothing has written yet
                    text = read CF_UNICODETEXT
                    if text == sentinel:        continue   // shouldn't happen
                    if !text.startsWith("Item Class: "):
                        // someone else wrote to the clipboard mid-read
                        record diagnostic; continue
                    return Fresh(text)

 5. VERIFY        Freshness is proven by the conjunction:
                    (a) the sentinel is gone,                 AND
                    (b) the sequence number moved past seq0,  AND
                    (c) the text starts with "Item Class: ".
                  (a) alone is sufficient in principle; (b) and (c) are cheap
                  and they separate the failure modes for diagnostics.

 6. TIMEOUT       On deadline: return Timeout. NEVER fall through to the old
                  text. FAIL CLOSED — see "The gate must fail closed" below.

 7. DO NOT RESTORE the user's previous clipboard. poe-graft never pastes into
                  the game, so there is nothing to restore for, and a restore
                  write adds a second clipboard mutation that races the next
                  read. (APT restores, and its own source warns that a lagging
                  game can then read the restored value — potentially a
                  password. We have no reason to take that risk.)
```

### The gate must fail closed

This is the design-shaping consequence, and it is not really about the clipboard:

- The moment a click is admitted, the app's knowledge of the item is **stale by construction** —
  the roll it was holding has been consumed. The gate state must immediately become
  `blocked (unknown)`, not `allowed`, and only a *successful* fresh read may reopen it.
- A `Timeout` must leave the gate blocked and surface loudly. Admitting a click after a timed-out
  read is precisely the over-roll the app exists to prevent.
- **Read latency, not human reaction time, becomes the roll-rate ceiling.** A human spamming
  alterations clicks roughly every 150–250 ms. A read costs on the order of 60–150 ms typically and
  up to the timeout in the bad case, so the gate will sometimes be closed when the human is ready to
  click. That is the correct behaviour but it is a felt cost, and the release/feedback design must
  make "blocked because I'm still reading" distinguishable from "blocked because you hit it".
- Do not attempt to overlap reads. One read in flight at a time (APT serializes on a single
  `pollPromise` for the same reason).

---

## Verbatim sample text

### A. Rare Ghastly Eye Jewel — real capture, our exact base

Verbatim from
[`timeloop-vault/poe-inspect` → `fixtures/items/rare-abyss-jewel-ghastly-eye.txt`](https://github.com/timeloop-vault/poe-inspect/blob/main/fixtures/items/rare-abyss-jewel-ghastly-eye.txt).
That repo's `fixtures/items/COVERAGE.md` documents the capture procedure as
"In-game: hover item → Ctrl+Alt+C (advanced copy with mod headers) → paste into a new `.txt` file",
i.e. these are real in-game captures, not hand-written.

```
Item Class: Abyss Jewels
Rarity: Rare
Ancient Globe
Ghastly Eye Jewel
--------
Abyss
--------
Requirements:
Level: 52
--------
Item Level: 69
--------
{ Prefix Modifier "Sparking" (Tier: 5) — Damage, Elemental, Lightning, Minion }
Minions deal 2(1-2) to 27(26-32) additional Lightning Damage
{ Prefix Modifier "Healthy" (Tier: 3) — Life }
+29(26-30) to maximum Life
{ Suffix Modifier "of Tolerance" (Tier: 2) — Chaos, Ailment }
34(31-40)% chance to Avoid being Poisoned
{ Suffix Modifier "of Stifling" (Tier: 1) — Attack, Minion }
Minions have 5(5-6)% chance to Blind on Hit with Attacks
(Being Blinded causes 20% less Accuracy Rating and Evasion Rating, for 4 seconds)
--------
Place into an Abyssal Socket on an Item or into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.
```

**Confidence: high.** Two independent cross-checks of this sample against
[Path of Building's mod data](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/master/src/Data/ModJewelAbyss.lua)
both reconcile exactly (see [Tier numbering](#tier-numbering-is-absolute-not-item-level-relative)),
which would be very unlikely for a fabricated sample. Line endings in the real clipboard are CRLF
(`\r\n`); normalise before splitting.

### B. Magic jewel with a prefix and a suffix — real capture

Verbatim from
[`timeloop-vault/poe-inspect` → `fixtures/items/magic-jewel-cobalt.txt`](https://github.com/timeloop-vault/poe-inspect/blob/main/fixtures/items/magic-jewel-cobalt.txt).
This is the *shape* poe-graft will actually see on every roll: `Rarity: Magic`, a single generated
name line, no `Requirements:` section, at most two explicit mods.

```
Item Class: Jewels
Rarity: Magic
Cerebral Cobalt Jewel of Grounding
--------
Item Level: 50
--------
{ Prefix Modifier "Cerebral" (Tier: 1) — Mana }
2(2-3)% increased Mana Reservation Efficiency of Skills
{ Suffix Modifier "of Grounding" (Tier: 1) — Elemental, Lightning, Resistance }
+12(12-15)% to Lightning Resistance
--------
Place into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.
```

Corroborating captures with identical grammar from two unrelated repos:

- [`bigbes/lootfilter` → `itemparser/assets/jewel-magic.txt`](https://github.com/bigbes/lootfilter/blob/master/itemparser/assets/jewel-magic.txt)
  (`Volleying Viridian Jewel of Atrophy`, `{ Prefix Modifier "Volleying" (Tier: 1) — Attack, Speed }`)
- [`maxensas/xiletrade` → `src/Xiletrade.Test/ItemInfoDescription/English/jewel-rare.txt`](https://github.com/maxensas/xiletrade/blob/master/src/Xiletrade.Test/ItemInfoDescription/English/jewel-rare.txt)
- [Path of Building's parser spec](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/master/spec/System/TestItemParse_spec.lua)
  — the highest-trust source available, since PoB's import path is exercised by the whole community:
  `{ Prefix Modifier "Freezing" (Tier: 5) — Damage, Elemental, Cold, Caster  — 8% Increased }` /
  `Adds 17(16-20) to 35(30-36) Cold Damage to Spells`

### C. What the v1 target roll will look like — RECONSTRUCTED, not captured

No public fixture exists for a magic Ghastly Eye Jewel carrying the target mod. The block below is
**assembled** from sample B's grammar plus verified mod data, so treat it as a test fixture to
*confirm* on device, not as evidence.

The target — `Minions deal (23-26) to (33-39) additional Physical Damage` — is
`AbyssMinionAddedPhysicalDamageJewel6` in
[`ModJewelAbyss.lua`](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/master/src/Data/ModJewelAbyss.lua#L100):
`type = "Prefix"`, `affix = "Flaring"`, `level = 83`, `group = "MinionAddedPhysicalDamage"`,
`modTags = { "physical_damage", "damage", "physical", "minion" }`. Its group has six tiers, so its
displayed tier is **1**. The map's `Tier 1 (23-26 … 33-39)` is therefore exactly this mod, and its
in-game affix name is **"Flaring"**.

```
Item Class: Abyss Jewels
Rarity: Magic
Flaring Ghastly Eye Jewel
--------
Abyss
--------
Item Level: 84
--------
{ Prefix Modifier "Flaring" (Tier: 1) — Damage, Physical, Minion }
Minions deal 24(23-26) to 36(33-39) additional Physical Damage
--------
Place into an Abyssal Socket on an Item or into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.
```

Uncertain parts of the reconstruction, all cheap to check on device:

- the tag list rendering. PoB stores `physical_damage, damage, physical, minion`; the game renders a
  deduplicated display form. Sample A's lightning analogue stores
  `elemental_damage, damage, elemental, lightning, minion` and renders
  `Damage, Elemental, Lightning, Minion`, so `Damage, Physical, Minion` is the consistent guess.
- whether `Requirements:` / `Level:` appears — sample A has one at ilvl 69 with four mods; sample B
  (a non-abyss jewel) has none. Do not assume a fixed section count.

---

## Detailed findings

### 1. Text format

`Ctrl+C` over a hovered item produces plain Unicode text on the clipboard (`CF_UNICODETEXT`),
CRLF-terminated lines, structured as `--------`-delimited sections:

| Section | Contents |
| --- | --- |
| 1 (name plate) | `Item Class: <class>`, `Rarity: <rarity>`, then 1–2 name lines. Rare/unique: rare name + base type on separate lines. Magic: **one** line combining prefix name + base + suffix name (`Cerebral Cobalt Jewel of Grounding`). Optionally a "you cannot use this item" line. |
| optional | Item-class subheading — for abyss jewels, the single line `Abyss` |
| optional | `Requirements:` followed by `Level: N`, `Str: N`, … |
| optional | `Sockets: …`, gem/armour/weapon property blocks, `Stack Size: …` for currency |
| one | `Item Level: N` |
| optional | enchant mods, then scourge mods, then implicit mods, then **explicit mods** — each as its own section |
| optional | flavour text, `Note: …` (price note), `Corrupted`, `Mirrored`, `Fractured Item`, `Unidentified`, `Split` markers |
| last | the base type's help text (`Place into an allocated Jewel Socket…`) |

Authoritative section-splitting reference:
[`Parser.ts` → `itemTextToSections`](https://github.com/SnosMe/awakened-poe-trade/blob/master/renderer/src/parser/Parser.ts)
splits on `/\r?\n/`, drops a trailing empty line, and cuts a new section on every line exactly equal
to `--------`. Section order is *not* guaranteed, so parse by content, not by index — APT runs a
list of parsers over the remaining sections and removes each section as it is claimed.

Language detection is by first line. APT keys off
`text.startsWith('Item Class: ')` for English
([`HostClipboard.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/shortcuts/HostClipboard.ts)),
with a table of ten localized equivalents (`Gegenstandsklasse: `, `Класс предмета: `, …). poe-graft
should require English and say so, exactly as `m4iraki/poe-crafting` does in its README
("Язык: Только English — парсер настроен на английские регулярные выражения").

### 2. Prefix vs suffix, and tier — both are in the text

Every explicit/implicit mod is preceded by an annotation line wrapped in braces:

```
{ <Type> "<AffixName>" (Tier: <n>) — <Tag>, <Tag>, … — <p>% Increased }
```

- The separator between segments is an **EM DASH `—`**, not a hyphen. APT splits on `'—'`
  ([`advanced-mod-desc.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/renderer/src/parser/advanced-mod-desc.ts)).
- `<Type>` is one of (English strings from
  [`renderer/public/data/en/client_strings.js`](https://github.com/SnosMe/awakened-poe-trade/blob/master/renderer/public/data/en/client_strings.js)):
  `Prefix Modifier`, `Suffix Modifier`, `Master Crafted Prefix Modifier`,
  `Master Crafted Suffix Modifier`, `Fractured Prefix Modifier`, `Fractured Suffix Modifier`,
  `Implicit Modifier`, `Corruption Implicit Modifier`, plus eldritch/foulborn/vestigial variants.
  **So yes — prefix and suffix are explicit and unambiguous.**
- APT's authoritative regex for the whole line:
  ```js
  MODIFIER_LINE: /^(?<type>[^"]+)(?:\s+"(?<name>[^"]*)")?(?:\s+\(Tier: (?<tier>\d+)\))?(?:\s+\(Rank: (?<rank>\d+)\))?$/
  ```
  Note `name` and `tier` are both optional: implicits and master crafts carry `(Rank: n)` instead of
  `(Tier: n)`, and some annotations have neither.
- `— <p>% Increased` is an optional third segment (mod magnitude increased, e.g. by catalysts). PoB's
  spec has a real example. If present, the printed ranges are the *scaled* ranges.
- **Roll and range are both printed**, inline, per number:
  `Minions deal 2(1-2) to 27(26-32) additional Lightning Damage` — value `2`, this tier's range
  `1-2`; value `27`, range `26-32`. This means the app can validate a hit against the tier's own
  range straight out of the clipboard, without any mod database, as a belt-and-braces check on its
  own tier table.

#### Tier numbering is absolute, not item-level-relative

This matters, because if tiers were relative the app could not key on `Tier: 1`. Two independent
reconciliations of sample A against PoB's mod data:

| Annotation in sample A | PoB entry | Group size | Index from bottom | `size - index + 1` | Matches? |
| --- | --- | --- | --- | --- | --- |
| `"Sparking" (Tier: 5)` | `AbyssMinionAddedLightningDamageJewel2` (ilvl 39) | 6 (`Humming`…`Electrocuting`) | 2 | **5** | yes |
| `"Healthy" (Tier: 3)` | `AbyssJewelAddedLife2` (ilvl 35) | 4 (`Hale`…`Stalwart`) | 2 | **3** | yes |

Sample A's jewel is `Item Level: 69`, at which `Discharging` (ilvl 70) and `Electrocuting` (ilvl 82)
cannot roll. Had the game numbered tiers relative to what the item level permits, `Sparking` would
have printed `Tier: 3`. It printed `Tier: 5`. **The displayed tier counts down from the highest tier
that exists for the mod group on that base type, regardless of the item's level.** A `Tier: 1` on a
low-ilvl jewel is therefore impossible rather than misleading, and matching on `Tier: 1` is safe.

### 3. The "advanced mod descriptions" setting — and why 3.29 changed it

Historically the annotation lines only appeared if you held the *Show Advanced Item Descriptions* key
while copying, hence the ubiquitous `Ctrl+Alt+C` in older tools.

- The key is configurable and lives in the game's own config file. APT reads
  `production_Config.ini` → `[ACTION_KEYS]` → `show_advanced_item_descriptions`, defaulting to `Alt`
  ([`GameConfig.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/host-files/GameConfig.ts)).
  On Windows the file is at `Documents\My Games\Path of Exile\production_Config.ini`.
- A GGG patch note (quoted in community documentation) reads: *"The 'Compare Item Descriptions' and
  'Show Advanced Item Descriptions' behaviour has been split from the 'Highlight Items and Objects'
  keybind into their own dedicated options. These new options default to holding Ctrl and Alt
  respectively. These new binds can be configured in the Options panel, under the Input section."*
- **In 3.29 this stopped mattering for copying.** Awakened PoE Trade removed the modifier press on
  2026-07-25 in commit
  [`cdad94cb`](https://github.com/SnosMe/awakened-poe-trade/commit/cdad94cb) ("no longer need to
  press adanced mod button #1865") with the in-source comment:
  > `// 3.29: Copying an item's text now always copies the advanced description format.`

  Corroborated by GGG's own
  [3.29.0b patch notes](https://www.pathofexile.com/forum/view-thread/3989412): *"Fixed a bug where
  using Ctrl+C on an item linked in chat was not copying the advanced description information."* —
  a bug report that only makes sense if plain `Ctrl+C` is expected to yield advanced information.
- The motivation for removing it was concrete: holding the advanced-descriptions modifier now
  **pins the in-game tooltip** (3.29 added PoE 2's "keywording and popup pinning system", per the
  [3.29.0 patch notes](https://www.pathofexile.com/forum/view-thread/3985332)), leaving stuck popups
  all over the screen — [APT issue #1865](https://github.com/SnosMe/awakened-poe-trade/issues/1865).

**Recommendation:** send plain `Ctrl+C`. Do not hold Alt or Ctrl-as-advanced-key. But make the
annotation lines' presence a *checked precondition*: if the first read of a session returns text with
no `{` line, tell the user to enable *Options → UI → Advanced Mod Descriptions* rather than silently
matching on stat text alone. (`m4iraki/poe-crafting`'s README still instructs users to enable that
checkbox; belt and braces costs nothing.)

### 4. Preconditions

| Precondition | Verdict | Source |
| --- | --- | --- |
| PoE window must be **foreground/focused** | **Required.** | Lailloken's `Exile-UI` explicitly activates the client first, with the comment *"activate the game-client in case it's not active (item-info cannot be copied from an inactive client)"* — [`modules/item-checker.ahk`](https://github.com/Lailloken/Exile-UI/blob/main/modules/item-checker.ahk). PoE Overlay calls `this.game.focus()` before copying. APT only registers its shortcuts while the game window is active (`poeWindow.on('active-change')` → `register()`/`unregister()`), and its troubleshooting page lists "Path of Exile must have focus" first. |
| Cursor must be **over the item** | **Required** — the copy targets the hovered item. Automated tools `MouseMove` to the item's centre and sleep one frame before sending the key (`Util.MClick`: `MouseMove` → `Sleep(FPSDelay=30ms)` → act). For poe-graft the human's cursor is already there, but the app must not move it. |
| Works with a **currency orb held in apply mode** on the cursor | **Very likely yes**, needs a device check. `m4iraki/poe-crafting` is an alteration-spam framework whose whole loop is: right-click the orb stack, left-click the item, `Sleep(80ms)`, then `MouseMove` + `Send("^!c")` + `ClipWait` — it never cancels apply mode before copying ([`lib/Stash.ahk`](https://github.com/m4iraki/poe-crafting/blob/master/lib/Stash.ahk), [`lib/Core.ahk`](https://github.com/m4iraki/poe-crafting/blob/master/lib/Core.ahk)). It also reads the *currency stack's* own tooltip the same way. That tool would not function if apply mode suppressed the copy. Flagged for verification anyway — it is the single assumption whose failure would sink the design. |
| Advanced descriptions enabled | Not needed for the annotation lines as of 3.29 (see §3), but worth asserting defensively. |
| Administrator rights | Only if PoE itself runs elevated. `SendInput` "is subject to UIPI. Applications are permitted to inject input only into applications that are at an equal or lesser integrity level" ([MS Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)), and *"neither GetLastError nor the return value will indicate the failure was caused by UIPI blocking"* — i.e. the failure is silent. APT's troubleshooting page lists admin rights as a fix. |
| No conflicting global hotkey | `Ctrl+C` is reserved by the game; APT refuses to *register* `Ctrl+C` as one of its own hotkeys and logs an error. Third-party global hooks (APT names ASUS GPU Tweak II, Radeon Software, BenQ Display Pilot, stray AutoHotkey scripts) can swallow the keystroke before the game sees it ([APT troubleshooting](https://snosme.github.io/awakened-poe-trade/nothing-happens.html)). |

### 5. Timing

**No published measurement of the real latency exists.** What exists is four independent tools'
*budgets*, which bound it:

| Tool | Clear first? | Wait strategy | Total budget |
| --- | --- | --- | --- |
| Awakened PoE Trade ([`HostClipboard.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/shortcuts/HostClipboard.ts)) | yes, if current content is item text | poll every `POLL_DELAY = 48` ms; first poll at 48 ms | `POLL_LIMIT = 500` ms, then reject with `Reading clipboard timed out` |
| PoE Overlay Community Fork ([`item-clipboard.service.ts`](https://github.com/PoE-Overlay-Community/PoE-Overlay-Community-Fork/blob/master/src/app/shared/module/poe/service/item/item-clipboard.service.ts)) | **no** | robotjs keyboard delay 25 ms, then retry the read up to 8 times with 25 ms delay | ~200 ms |
| Lailloken Exile-UI ([`item-checker.ahk`](https://github.com/Lailloken/Exile-UI/blob/main/modules/item-checker.ahk)) | yes (`Clipboard := ""`) | `SendInput ^{c}` then `ClipWait, 0.1` | 100 ms |
| m4iraki/poe-crafting ([`Core.ahk`](https://github.com/m4iraki/poe-crafting/blob/master/lib/Core.ahk)) | yes (`A_Clipboard := ""`) | `Sleep(30ms)` after `MouseMove`, `Send("^!c")`, `ClipWait(0.5)`; 3 attempts, 150 ms apart | 500 ms/attempt, ~2 s worst case |

Reading these together:

- APT's first probe is at **48 ms** and its 500 ms ceiling is described in its own code as the point
  at which "the game lagged for some reason". Exile-UI ships a **100 ms** ceiling as its normal
  operating budget. So the typical latency is **well under 100 ms**, and plausibly under 50 ms.
- APT deliberately does not restore the clipboard for **120 ms** (`RESTORE_AFTER`) after a write it
  wants the game to read, with the comment *"PoE must read clipboard within this timeframe."* That is
  APT's own estimate of the game's clipboard-interaction window and is the best single number
  available for how long the game can lag.
- Latency scales with **frame time**, not network ping: m4iraki separates `FPSDelay` ("задержка на
  отрисовку" — rendering delay, default 30 ms, "increase if the game stutters") from `PingDelay`, and
  its error message for a failed copy is *"increase PingDelay … or check FPS"*. Expect the tail to
  fatten during heavy load / a stuttering client.
- **Recommendation: 400 ms deadline, poll every 10–15 ms.** Poll faster than APT — 48 ms granularity
  costs up to 48 ms of pure gate-closed time per read, which is a meaningful fraction of a human
  click interval. Prefer `AddClipboardFormatListener` → `WM_CLIPBOARDUPDATE` as the primary wakeup
  (the docs recommend a listener over polling and explicitly say the sequence number "is not a
  notification method and should not be used in a polling loop") with a short timer as a backstop,
  since the listener alone cannot detect "nothing ever happened".

**Measuring the real distribution is an open item and should be instrumented in v1** — the app is
already going to need on-device diagnostics (per the map), so log `(seq0, seq_at_success, elapsed_ms,
poll_count)` for every read. That log is the only way this number will ever be known on that
machine.

### 6. Staleness — the failure modes, and why the sentinel is necessary

Failure modes that leave old text on the clipboard:

1. **The keystroke never reached the game.** Window lost focus between the check and the send; UIPI
   blocked `SendInput` silently; a third-party global hook ate `Ctrl+C`; `SendInput` returned 0
   because "the input was already blocked by another thread" (something called `BlockInput`).
2. **The game received it but could not write the clipboard.** `OpenClipboard` *"fails if another
   window has the clipboard open"* ([MS Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openclipboard)).
   If poe-graft — or a clipboard manager — holds the clipboard at that instant, the game's copy fails
   and nothing is written. This is a self-inflicted wound we can avoid by keeping our own
   open/close windows minimal.
3. **The cursor was not over an item** (the human moved between roll and read) — nothing is copied.
4. **The game lagged past our deadline** and writes *after* we gave up — the write then lands during
   the *next* read, which the sequence-number check will see as an out-of-band mutation. Because the
   text is still valid item text this is the nastiest case: it is text for the *previous* roll
   arriving late. The sentinel does not distinguish it. **Mitigation: one read in flight at a time,
   and on `Timeout` re-poison the clipboard before the next read so any late write is superseded.**

Why content comparison is insufficient, restated precisely: two consecutive alteration rolls on a
one-mod magic jewel can produce byte-identical text (same affix, same tier, same roll value), so
`text != previous_text` neither proves nor disproves freshness. Nor does the item *name* help — the
magic name is derived from the affixes and repeats with them.

Why the sentinel is sufficient: the game only ever writes text beginning with `Item Class: `. A
sentinel like `__POE_GRAFT_00417_1754300000123456789` is not producible by the game, so
"the clipboard no longer holds my sentinel and does hold `Item Class: …`" is proof that *something
wrote item text after my poison*. Combined with one-read-at-a-time and re-poisoning on timeout, the
only writer that could have done so is the game responding to this read.

Why keep `GetClipboardSequenceNumber` as well:

- It is the only signal that distinguishes *"nothing wrote at all"* (failure mode 1/3 — seq
  unchanged) from *"someone else wrote"* (a clipboard manager, the user's own Ctrl+C — seq changed
  but content is not item text). On a machine with no debugger, that distinction is the difference
  between "your hotkey is being stolen" and "your clipboard manager is fighting us".
- It is cheap: no `OpenClipboard`, no ownership, no contention. Semantics: *"The system keeps a serial
  number for the clipboard for each window station. This number is incremented whenever the contents
  of the clipboard change or the clipboard is emptied."* Returns **0** if the process lacks
  `WINSTA_ACCESSCLIPBOARD` — treat a `0` return as "sequence checking unavailable" and fall back to
  the sentinel alone rather than misreading 0 as a real value.
- Caveat: *"If clipboard rendering is delayed, the sequence number is not incremented until the
  changes are rendered."* If PoE used delayed rendering (`SetClipboardData(fmt, NULL)`), the seq
  number would lag. No evidence it does — every tool surveyed reads the text immediately and
  succeeds — but this is why the sentinel, not the sequence number, is the *primary* witness.
- Note the count is not necessarily +1: `EmptyClipboard` and `SetClipboardData` each mutate, so a
  single game-side copy may advance the sequence by more than one. Compare with `!=`, never `== seq0+1`.

For contrast, **PoE Overlay's approach is the bug this ticket is about**: it snapshots `oldText`,
sends the key, reads, and only retries when the result is *empty* — it never clears first. If the
game's copy fails while item text from a previous read is on the clipboard, PoE Overlay happily
parses the stale item. Do not copy that design.

### 7. Synthesized Ctrl+C — does it reach the game?

**Yes, reliably, via `SendInput`.** All four surveyed tools use it and work:

- APT uses `uiohook-napi` → libuiohook, whose Windows backend builds `INPUT` structures and calls
  `SendInput` ([`src/windows/post_event.c`](https://github.com/kwhat/libuiohook/blob/master/src/windows/post_event.c)).
- Exile-UI uses AutoHotkey's `SendInput` mode explicitly (`SendInput, ^{c}`).
- PoE Overlay uses robotjs (`keyTap`), also `SendInput`-based.

`SendInput` events *"are not interspersed with other keyboard or mouse input events inserted either
by the user … or by calls to keybd_event, mouse_event, or other calls to SendInput"* — the Ctrl+C
sequence arrives atomically, which is what makes this safe to fire while the human is mid-click-spam.
The game reading raw input is not an obstacle: `SendInput` injects into the same system input stream.

Known caveats, all of which the prior art handles explicitly:

- **Key-state pollution.** *"This function does not reset the keyboard's current state. Any keys that
  are already pressed when the function is called might interfere with the events that this function
  generates. To avoid this problem, check the keyboard's state with the GetAsyncKeyState function and
  correct as necessary."* Both APT and PoE Overlay do exactly this: APT releases the non-modifier key
  of the triggering hotkey (`uIOhook.keyToggle(key, 'up')`) before synthesizing, and filters out
  modifiers that are already physically held so it does not release a key the user is holding; PoE
  Overlay force-releases both Alt keys (`VK_LMENU`, `VK_RMENU`) before its `keyTap`. poe-graft's
  trigger is a mouse click, not a hotkey, so the risk is smaller — but the human *may* be holding
  Shift (to apply currency without the confirmation prompt) or Ctrl. Read `GetAsyncKeyState` and
  either release-and-restore or abort, and never leave a modifier down that you pressed.
- **UIPI/elevation** — see §4.
- **`SendInput` returning 0** means the input stream was blocked by another thread; treat as a read
  failure, not a timeout.
- **Do not synthesize Alt** — see §3, it pins tooltips in 3.29.

### 8. Rate limits and in-game cost of copying

- **No server round trip.** The item's mod text is already resident in the client (it is what the
  tooltip renders), so `Ctrl+C` is a purely local operation. This is the consensus in the
  [r/pathofexile thread on exactly this kind of over-roll-prevention script](https://www.reddit.com/r/pathofexile/comments/10hfx0d/i_made_a_script_that_prevents_you_from_rolling/)
  — *"Ctrl+C does not communicate with the server — it just moves information already on the client
  to your clipboard"* and *"copy/paste, reading clipboard and blockinput have nothing to do with GGG
  servers"*. Confidence: high on mechanism, medium on the absence of any hidden client-side throttle.
  (Reddit is not directly fetchable from this environment; the quotes are from search-result
  extracts of that thread and should be treated as community claims, not documentation.)
- **The one documented throttle in the prior art is unrelated to copying.** APT's `restoreShortly`
  carries the comment *"This throttling helps against disconnects from 'Too many actions'"* — but
  that path is for **pasting into chat / stash search**, i.e. actions that *do* hit the server. It is
  not on the item-copy path.
- **The real budget is frame time**, per §5. m4iraki's alteration loop runs an unbounded number of
  copies (`MaxAttempts` defaults to 100 000) with no rate limiting beyond its frame/ping sleeps, which
  is direct evidence that hundreds-to-thousands of copies per session are fine.
- poe-graft issues at most one copy per human click, so a few per second at the very most. Well
  inside any plausible limit.

### 9. Alternatives to the clipboard

| Alternative | Verdict |
| --- | --- |
| **`Client.txt` game log** | **Does not carry item data.** APT watches it ([`GameLogWatcher.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/host-files/GameLogWatcher.ts), `logs/Client.txt`, polled with `watchFile` at a 450 ms interval) purely for area/zone changes and chat lines. Even if it did, a 450 ms poll interval is an order of magnitude too slow. Dead end. |
| **Local API / memory reading** | No local HTTP API exists. Memory reading is what actual botting frameworks do and would put the project squarely on the wrong side of the map's TOS line. Ruled out. |
| **GGG public API (stash tabs)** | The stash-tab API does return full mod data for items, but it is a rate-limited HTTP endpoint on GGG's servers with per-account limits measured in requests per minute, and it reflects server state after a delay. Utterly unusable at ~4 reads/second. Ruled out on latency alone. |
| **OCR** | Already the map's declared fallback, not v1. Note that APT ships OCR (`vision/HeistGemFinder.ts`) *only* for Heist gem rewards — a case where no copy exists at all — which is a good signal that OCR is not chosen when a clipboard path is available. |
| **`production_Config.ini`** | Not an item source, but a genuinely useful read: it is where the advanced-descriptions keybind lives, so poe-graft can *verify* the user's configuration instead of asking. See §3. |

The clipboard is the only viable mechanism. That was already the map's decision; this research
confirms it and finds no reason to revisit it.

---

## Open questions requiring the game

Each of these can only be settled by hovering a real item in a real client. None are guesses I have
smuggled into the findings above.

1. **Does `Ctrl+C` copy the hovered item while an Orb of Alteration is held in apply mode on the
   cursor?** The critical one — the entire design assumes yes. Evidence is strong (§4) but indirect.
   *Check:* right-click alts in stash, hover the jewel, `Ctrl+C`, inspect the clipboard. Also check
   the variant where the human holds **Shift** (apply-without-confirmation), since that adds a held
   modifier during the synthesized keystroke.
2. **Actual latency distribution** from `SendInput` to clipboard-contains-item-text: median, p99, and
   behaviour during a frame-rate dip. *Check:* instrument the read (§5) and log 500 reads from a real
   alt-spam session.
3. **Is the annotation format really produced by plain `Ctrl+C` in 3.29 without the Advanced Mod
   Descriptions checkbox** in Options → UI, or does the checkbox still gate it? APT's code and GGG's
   3.29.0b note both point to "always advanced", but the two settings (the hold-key and the
   always-on checkbox) have historically been separate and I could not find the 3.29 patch line that
   changed copy behaviour. *Check:* toggle the checkbox off and copy.
4. **Verbatim text of a magic Ghastly Eye Jewel carrying `Flaring`** — the reconstruction in sample C
   must be replaced with a real capture. Specifically: the exact tag list rendering
   (`Damage, Physical, Minion`?) and whether a `Requirements:` section appears.
5. **Does the game ever write a partial/empty clipboard** (e.g. `EmptyClipboard` succeeded but
   `SetClipboardData` failed), which would advance the sequence number with no text? If so the poll
   loop must not treat "seq changed, empty text" as terminal. Cheap to make robust regardless; worth
   knowing whether it happens.
6. **Does hovering the item and copying while the tooltip is *pinned*** (the new 3.29 popup-pinning
   system) change what is copied, or copy the pinned popup's item instead of the hovered one? A
   stuck pinned popup is a plausible accident during a long crafting session.
7. **Any hidden client-side throttle on copying** at ~4/s sustained for thousands of copies. No
   evidence of one; worth watching the diagnostics log for a rising timeout rate over a long session.

---

## Source index

**Tool source code (primary):**

- Awakened PoE Trade — [`main/src/shortcuts/HostClipboard.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/shortcuts/HostClipboard.ts),
  [`main/src/shortcuts/Shortcuts.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/shortcuts/Shortcuts.ts),
  [`main/src/host-files/GameConfig.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/host-files/GameConfig.ts),
  [`main/src/host-files/GameLogWatcher.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/host-files/GameLogWatcher.ts),
  [`renderer/src/parser/advanced-mod-desc.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/renderer/src/parser/advanced-mod-desc.ts),
  [`renderer/src/parser/Parser.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/renderer/src/parser/Parser.ts),
  [`renderer/public/data/en/client_strings.js`](https://github.com/SnosMe/awakened-poe-trade/blob/master/renderer/public/data/en/client_strings.js),
  commit [`cdad94cb`](https://github.com/SnosMe/awakened-poe-trade/commit/cdad94cb),
  [issue #1865](https://github.com/SnosMe/awakened-poe-trade/issues/1865),
  [troubleshooting page](https://snosme.github.io/awakened-poe-trade/nothing-happens.html)
- PoE Overlay Community Fork — [`item-clipboard.service.ts`](https://github.com/PoE-Overlay-Community/PoE-Overlay-Community-Fork/blob/master/src/app/shared/module/poe/service/item/item-clipboard.service.ts),
  [`electron/robot.ts`](https://github.com/PoE-Overlay-Community/PoE-Overlay-Community-Fork/blob/master/electron/robot.ts)
- Lailloken Exile-UI — [`modules/item-checker.ahk`](https://github.com/Lailloken/Exile-UI/blob/main/modules/item-checker.ahk)
- m4iraki/poe-crafting (AHK v2 alteration-spam framework) — [`lib/Core.ahk`](https://github.com/m4iraki/poe-crafting/blob/master/lib/Core.ahk),
  [`lib/Stash.ahk`](https://github.com/m4iraki/poe-crafting/blob/master/lib/Stash.ahk),
  [`lib/Config.ahk`](https://github.com/m4iraki/poe-crafting/blob/master/lib/Config.ahk),
  [`lib/AlterationCrafting.ahk`](https://github.com/m4iraki/poe-crafting/blob/master/lib/AlterationCrafting.ahk)
- libuiohook — [`src/windows/post_event.c`](https://github.com/kwhat/libuiohook/blob/master/src/windows/post_event.c)
- Path of Building Community — [`spec/System/TestItemParse_spec.lua`](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/master/spec/System/TestItemParse_spec.lua),
  [`src/Data/ModJewelAbyss.lua`](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/master/src/Data/ModJewelAbyss.lua)

**Item-text fixtures:**

- [`timeloop-vault/poe-inspect` fixtures](https://github.com/timeloop-vault/poe-inspect/tree/main/fixtures/items) (68 captures, documented capture procedure)
- [`bigbes/lootfilter` itemparser assets](https://github.com/bigbes/lootfilter/tree/master/itemparser/assets)
- [`maxensas/xiletrade` test descriptions](https://github.com/maxensas/xiletrade/tree/master/src/Xiletrade.Test/ItemInfoDescription/English)

**Microsoft Learn:**

- [`GetClipboardSequenceNumber`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardsequencenumber)
- [`AddClipboardFormatListener`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-addclipboardformatlistener)
- [`OpenClipboard`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openclipboard)
- [`SendInput`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
- [Using the Clipboard](https://learn.microsoft.com/en-us/windows/win32/dataxchg/using-the-clipboard)

**GGG:**

- [Content Update 3.29.0 — Curse of the Allflame](https://www.pathofexile.com/forum/view-thread/3985332)
- [3.29.0b Patch Notes](https://www.pathofexile.com/forum/view-thread/3989412)
- [3.29.1 Patch Notes](https://www.pathofexile.com/forum/view-thread/3991672)
