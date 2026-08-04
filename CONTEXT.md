# poe-graft

Over-roll protection for Path of Exile 1 crafting. The human spams Orbs of Alteration on one item;
poe-graft's job is to notice the moment the wanted modifier lands and stop the next roll from
destroying it.

This is the project's glossary. Terms defined here are used verbatim in code, logs, UI copy, tickets
and commit messages.

## Path of Exile crafting

**Alteration**:
An Orb of Alteration — the currency that rerolls a magic item's modifiers. One roll consumes one.
_Avoid_: alt, orb (unqualified)

**Apply Mode**:
The game state in which an armed currency is repeatedly applied to whatever is clicked, entered by
holding Shift with the currency on the cursor. It persists across clicks and draws from the
inventory, never a stash tab.
_Avoid_: Shift-persist, spam mode

**Base**:
The item type being crafted, independent of its modifiers — for v1, always Ghastly Eye Jewel.
_Avoid_: base type, item type

**Mod**:
One modifier on an item: a line of text plus the numbers rolled into it. Every mod is a **prefix**
or a **suffix**, and an alteration produces one or two of them.
_Avoid_: modifier, affix, stat

**Mod Group**:
The family a mod belongs to, spanning every tier of it. Its display name changes per tier — the
group behind *Minions deal # to # additional Physical Damage* is `Annealed` at T4 and `Flaring` at
T1 — so a group is never identified by a single name.
_Avoid_: mod family, affix group

**Tier**:
A mod group's power band, numbered from 1 (best) downwards, each with its own numeric ranges. Derived
from the numbers a mod rolled, never read from the game's own tier annotation.

**Item Text**:
What the game puts on the clipboard for the hovered item. Divided into sections; only the
**explicit-mod section** is ever consulted, which is what keeps an implicit or a corrupted mod from
being read as a **Hit**.
_Avoid_: tooltip, clipboard text, item info

## The craft

**Target Mod**:
The mod group the human is crafting for, together with a **Tier Threshold** — the worst tier that
still counts as success. Both are chosen before a **Craft Session** starts.

**Craft Session**:
One item, one Target Mod, one continuous attempt. It ends when the item **Latches**, when the app
**Halts**, or when the human stops.
_Avoid_: session (unqualified), run, craft

**Anchor**:
The single screen coordinate a Craft Session captures, where the item being crafted sits. The cursor
never moves; it only has to stay within tolerance of the Anchor.
_Avoid_: item position, click target, coordinate

**Trigger Key**:
The keyboard key the human taps to advance the craft. Suppressed while a Craft Session is armed so
the game never sees it, and left alone otherwise.

**Trigger Press**:
One physical, non-repeating press of the Trigger Key. The app acts only on these — never on a timer,
never on its own input, never twice for one press. Every action the app takes is attributable to
exactly one Trigger Press.
_Avoid_: keypress, trigger, tap

**Roll**:
One injected Shift+left-click on the Anchor: one Alteration spent. Counted the instant the click
lands, because the orb is gone whether or not anything is learned afterwards.
_Avoid_: click, attempt, spin

**Read**:
Poisoning the clipboard with a **Sentinel**, asking the game to copy, and retrieving what replaces
it. A Read costs no Alteration and can happen without a Roll.

**Sentinel**:
The unique text poe-graft writes to the clipboard before every Read, so that anything else found
there could only have come from the game. Two identical rolls produce identical Item Text, so
content alone can never prove freshness.
_Avoid_: poison, marker, token

## Verdicts

**Verdict**:
What a Read establishes about the item currently under the Anchor: a **Hit**, a **Miss**, or
**Unknown**.

**Hit**:
The explicit-mod section contains the Target Mod at or better than the Tier Threshold. Ambiguity
resolves to a Hit — the safe direction is always to stop.

**Miss**:
A Read that positively establishes the Target Mod is absent or out of tier. Only a fresh Miss
permits a Roll.

**Unknown**:
A Read that establishes nothing — the game never copied, nothing readable arrived, the text could
not be parsed, or the text is byte-identical to the previous Roll's and so is probably a photograph
taken before the game finished applying the orb.
_Avoid_: failed read, timeout, bad read

**Diagnostic**:
Something a Read establishes *besides* its Verdict — the game's tier annotation disagreeing with the
derived tier, an annotation being absent, values matching no tier, a line the mod pool does not
recognise, the item not being the Base. Every Diagnostic is logged; some of them are what make the
app **Halt**. A Diagnostic never changes a Verdict.
_Avoid_: warning, error, note

**Resync**:
A Trigger Press that performs a Read and no Roll, taken when the app's Verdict is Unknown. Its
result is authoritative because no Roll intervened, and it costs a press rather than an orb.
_Avoid_: retry, re-read, recovery

**Latch**:
The refusal to Roll again after a Hit. Released only by the human acknowledging it in poe-graft's own
window; no key can clear it.
_Avoid_: lock, block, gate, stop

**Halt**:
Stopping a Craft Session because the app can no longer trust what it is looking at — the item is not
the one the session began on, or three Reads in a row came back Unknown. Requires a deliberate
re-arm.
_Avoid_: disarm, abort, error

**Refusal**:
Declining to act on a Trigger Press without changing state, because a precondition of a Roll is
momentarily false: Shift is not held, Path of Exile is not the foreground window, or the cursor has
drifted off the Anchor. Nothing is spent and nothing is learned.
_Avoid_: ignore, drop, reject

## States

A Craft Session is always in exactly one of these. The invariant across all of them: **the app never
spends an Alteration without a fresh Miss for the item in front of it.**

**Idle**:
No Craft Session. The Trigger Key is not suppressed and the app cannot click.

**Sighting**:
Armed, with no Anchor and no Verdict. The next Trigger Press captures the Anchor and Reads — it does
not Roll.

**Ready**:
Holding a fresh Miss. The next Trigger Press Rolls and Reads.

**Rolling**:
A Roll and its Read are in flight. Further Trigger Presses are counted and dropped rather than
queued — with one piece of small print: the newest press to arrive during the cycle is served once the
cycle ends, so two quick taps are two actions rather than one. It cannot over-roll, because a press is
judged when it is served and Rolling is not Ready: it still takes a fresh Miss to Click.

**Resyncing**:
Holding an Unknown Verdict. The next Trigger Press Resyncs.

**Latched**:
A Hit has been found. Awaiting acknowledgement; survives losing focus.

**Halted**:
Stopped and untrusting. Awaiting re-arm.
