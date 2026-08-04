# ADR 0002 — The roll cycle: an unknown verdict costs a press, not an orb

- **Status:** Accepted
- **Date:** 2026-08-04
- **Deciding ticket:** [Design the roll cycle and the hit latch](https://github.com/Furizaa/poe-graft/issues/7)
- **Feeders:** [#15 the pivot](https://github.com/Furizaa/poe-graft/issues/15),
  [#16 Ctrl+C while armed](https://github.com/Furizaa/poe-graft/issues/16),
  [#17 the on-device spike](https://github.com/Furizaa/poe-graft/issues/17),
  [#18 the deaf hook](https://github.com/Furizaa/poe-graft/issues/18)
- **Vocabulary:** [`CONTEXT.md`](../../CONTEXT.md) — every capitalised term below is defined there

## Context

The app injects one Shift+left-click and one `Ctrl+C` per Trigger Press, reads the item, and on a Hit
refuses the next press ([#15](https://github.com/Furizaa/poe-graft/issues/15)). The spike proved that
mechanism on the gaming PC ([#17](https://github.com/Furizaa/poe-graft/issues/17)). What was left was
the cycle itself, and one thing the spike surfaced that reframes it.

**A fresh clipboard does not mean a fresh item.** The Sentinel protocol proves the *clipboard* changed
after we poisoned it. It cannot prove the Item Text describes the state *after* the Roll — applying
an Alteration is server-authoritative, and the tooltip is a photograph taken at copy time. Copy too
early and you get a clean, parseable, confident description of the item as it was *before* the orb
landed. That is the one remaining path to a silent over-roll, and it is the same fault as the spike's
40 ms failures wearing a disguise: at 40 ms the copy did not happen at all, which is loud; a slightly
later copy succeeds and lies.

So there are four ways to end a Read with nothing trustworthy — the game never copied, nothing
readable arrived, the text will not parse, and the text is identical to the previous Roll's — and they
are all the same thing: **the app does not know what the item is.** The spike treated that as
bookkeeping: it counted the failure and let the next press Roll regardless. At a 0.3591% per-click hit
rate each such press is a ~1-in-278 chance of having just destroyed the Hit it made.

Strict fail-closed — never Roll without a Verdict — was previously unaffordable, because the only way
to obtain a Verdict was to Roll again. That is no longer true. Since the pivot, the app acts *on a
press*, and a press does not have to mean a click.

## Decision

### An Unknown Verdict is resolved by the next press, without a Roll

When the app holds an Unknown Verdict it enters **Resyncing**. The next Trigger Press performs a Read
and **no Roll**: one `Ctrl+C`, no click, no orb. The result is authoritative by construction, because
no Roll intervened between the previous state and this Read — whatever the server has settled on is
what the tooltip now shows, identical text included.

This stays inside the TOS rule rather than stretching it: a Resync performs *fewer* actions than the
one-action-per-press budget allows, and it is still one physical press per action.

The cost is a press that does not Roll. That is the whole price of making over-rolling structurally
impossible, and at a tuned 130 ms settle delay Unknown Verdicts approach zero anyway
([#17](https://github.com/Furizaa/poe-graft/issues/17)).

### The invariant

**The app never spends an Alteration without a fresh Miss for the item in front of it.**

This is what the states exist to enforce, and it has a consequence that falls out for free: the first
press of a Craft Session Reads rather than Rolls. So the app also cannot roll an item it has never
looked at — including a jewel that already carries the Target Mod, which the spike would have rolled
straight past.

### The states

```
Idle ──arm──▶ Sighting ──press: Read only──▶ Ready ──press: Roll + Read──▶ Rolling
                                              ▲                              │
                                              │                     ┌────────┼────────┐
              Resyncing ──press: Read only────┘                    Miss     Hit    Unknown
                    ▲                                               │        │        │
                    └───────────────────────────────────────────────┘        ▼        ▼
                                                                        Latched   Resyncing
              Halted ◀── wrong item · tier-data disagreement · 3 consecutive Unknowns
```

`Sighting` and `Resyncing` behave identically on a press. They stay distinct because what the app has
to *say* differs — "hover the jewel and press" versus "press again, I lost track of the item" — and a
state the human cannot act on correctly is worse than an extra state name.

`Rolling` exists to drop presses rather than queue them. It lasts ~130–210 ms and is imperceptible;
nobody will ever experience it as being blocked.

### Halt versus Refusal

Two kinds of not-acting, deliberately not conflated:

| | |
| --- | --- |
| **Halt** — needs re-arm | The Item Text describes a different item than the session's first Read (base, item level or rarity differs); the parser and the tier data disagree; three consecutive Unknown Verdicts |
| **Refusal** — state unchanged | Shift is not held; the foreground window is not `POEWindowClass`; the cursor has drifted more than 24 px off the Anchor |

A Refusal is a momentarily false precondition the human fixes in a second. A Halt means the app cannot
trust its own eyes, and continuing would be guessing. Wrong-item is a Halt rather than an Unknown
precisely because Resyncing would re-read the same wrong item forever, turning every press into a dead
one while telling the human nothing.

Three consecutive Unknowns is tight where the spike's `BAD_LIMIT = 5` was loose, and it can afford to
be: with Resync in place a run of Unknowns spends **no orbs**, so the guard's job shrinks from damage
control to telling the human that the orb has left the cursor or the jewel is no longer hovered. A
false Halt costs one mouse click.

### The hit test reads numbers, not the game's tier annotation

A Hit requires, within the **explicit-mod section only**:

1. a mod line matching the Target Mod group's text template, and
2. rolled values falling inside the ranges of some tier at or better than the Tier Threshold,
   according to `data/ghastly-eye-jewel.json`.

Tier is **derived from the numbers**. The game's `{ Prefix Modifier "Flaring" (Tier: 1) … }`
annotation is parsed and logged as a cross-check, and never feeds the Verdict.

Three reasons, in order of weight. It generalises: multi-mod targets and other bases need per-mod
matching that name lookup tables do not give cheaply. The annotation is a **client setting** (Advanced
Mod Descriptions), so depending on it means the app breaks silently the day that setting is off — on a
new install, on another machine, after a client reset. And the annotation's mod name is *per tier* —
the group behind the target is `Annealed` at T4 and `Flaring` at T1 — so anything matching a single
name was going to be a bug regardless.

Section awareness is what the annotation was quietly buying: without `Prefix Modifier` to lean on,
"only the explicit-mod section counts" is what stops a corrupted implicit or an enchantment from ever
reading as a Hit. [#4](https://github.com/Furizaa/poe-graft/issues/4) established this base has no
overlapping tier ranges, so the derivation is exact rather than heuristic. If values match no tier at
all, or somehow match more than one, the app fails closed: call it a Hit and stop.

### The Latch is released with the mouse, and only in our own window

Acknowledging a Hit requires a mouse click on a control in poe-graft's window. No key releases it —
not the Trigger Key, not a dedicated hotkey. A key that can clear a Hit is a key your reflexes can
clear a Hit with, which would defeat the entire app on the one press that matters.

Acknowledging closes the Craft Session, recording its Roll count, and opens a new one in `Sighting`
with the **same Target Mod and Tier Threshold** — the realistic next action after a Hit is the next
jewel, not the same one. The Anchor, the pinned item identity and the Roll count are discarded. This
has a pleasing property: acknowledge and hover the *same* jewel, and the baseline Read immediately
re-Latches instead of rolling it.

The Latch survives losing focus for free — it is session state, unrelated to focus. It needs no
persistence across restarts either, because the app comes up `Idle` and `Idle` cannot click.

### The Trigger Key is `[`, suppressed only while armed

`[` (`VK_OEM_4`, `0xDB`) is the default: the only key with on-device evidence behind it, having run 60
rolls through the spike with suppression on while the game never saw it. F13–F24 are out — the owner's
keyboard has no F-keys — and [#17](https://github.com/Furizaa/poe-graft/issues/17) removed the need
for a key PoE ignores natively, since keyboard suppression is confirmed to reach the client.

Suppression is active **whenever a Craft Session is armed, and only then.** Scoping it to the
foreground window instead would require calling `GetForegroundWindow` inside the hook callback, which
the callback's rules forbid; armed-or-not is a relaxed atomic load, which they permit. `Idle`
therefore leaves the key alone system-wide. The foreground guard stays where the spike put it — on the
*action*, in the worker.

> **Correction, 2026-08-04** — [#20](https://github.com/Furizaa/poe-graft/issues/20) built this, and
> moved half of the last sentence.
>
> **The three Refusal preconditions are *read* in the worker and *judged* in `crates/core`.** They
> arrive as fields on a `Press` — `shift_down`, `foreground_class`, `cursor` — and `CraftSession`
> decides between a plan and a Refusal. The rule that mattered is untouched: the hook callback still
> does nothing but relaxed atomic loads, and `GetForegroundWindow` is still called on the worker
> thread. What changed is that every Refusal path is now a test on the development machine instead of
> something only the gaming PC could exercise.
>
> **"Armed" resolves to `state != Idle`**, which includes `Latched` and `Halted`. A Halt is not
> something the human notices instantly, and their next few reflex presses would otherwise leak `[`
> straight into the game mid-craft. `Idle` is still the only state that lets the key through.

### Feedback: three sounds, distinct in kind

**Hit** (loud and unmistakable), **Halt** (a warning), **Resync and Refusal** (a quiet blip meaning
"that press did not Roll"). `Ready` and `Rolling` are silent.

The blip is not polish. A Resync press is physically identical to a Roll press and the human cannot
see that the item did not change, so without it they would believe they had rolled. Which sounds, and
the window's layout, belong to
[Prototype the mod selection and craft session UI](https://github.com/Furizaa/poe-graft/issues/9).

### The cycle is a pure state machine in `crates/core`

`core` owns it as events in, commands out — no clock, no Win32, no I/O. A press in `Ready` produces a
command list along the lines of `[Poison(sentinel), Click, CountRoll, Settle(130), SendCopy,
AwaitRead(150)]`; `win32` executes commands and feeds back events. Timing values are core
configuration rather than `win32` atomics.

This is a departure from `spike.rs`, which holds its state in atomics and sleeps inline. The spike's
shape is proven, but it is only testable on the gaming PC — the one machine that cannot run tests. The
pure version makes the *whole* cycle testable on the Mac, which is exactly what the map's fog asked
for: replaying captured rolls through the cycle rather than only the parser. Every path — Resync, the
wrong-item Halt, the identical-text case, the Latch — becomes a sequence of `ReadResult` events in
`cargo test -p poe-graft-core`.

> **Correction, 2026-08-04** — built by
> [#20](https://github.com/Furizaa/poe-graft/issues/20) in `crates/core/src/cycle.rs`, and the sketched
> command list is one command shorter than it appears above.
>
> **There is no `CountRoll` command.** `win32` reports whether the click actually landed
> (`CycleReport { rolled, read }`) and core counts the Roll from that, so the count is what was really
> spent rather than what was asked for. The rule it exists to protect is unchanged: the Roll count
> advances because the click landed, never because a Read succeeded. The real plan for a press in
> `Ready` is `[Poison(sentinel), Click, Settle(130), SendCopy, AwaitRead(150, 80)]`, and for `Sighting`
> or `Resyncing` it is the same list without `Click` and `Settle`.
>
> One thing this ADR did not anticipate, found by the exhaustive invariant walk rather than by a test
> anyone thought to write: **a `CycleReport` arriving with no plan in flight has to be discarded.**
> Otherwise a duplicated report counts one Alteration twice, and one arriving after a `Disarm` drags a
> closed session back to life — which is the same shape as the spike's `DISARMED ITSELF`, then rolled
> 51 ms later.

### Settled defaults, in code

Measured on device ([#17](https://github.com/Furizaa/poe-graft/issues/17)), and they ship as code
defaults rather than as advice: **130 ms** settle delay between Click and `Ctrl+C`, **150 ms** Read
timeout, **80 ms** read-settle retry past the `EmptyClipboard` sequence-number bump, **24 px** Anchor
tolerance, suppression **on**, Unknown-run limit **3**.

### There is no roll cap

The spike's `MAX_ROLLS = 60` was a spike safety net and would fire every session against a median of
193 Alterations. The app cannot run away — it acts only on a physical press — and running *out* of
orbs self-halts: Apply Mode ends, the Item Text stops changing, identical text is an Unknown Verdict,
and three presses later the session is `Halted`. An orb budget, if ever wanted, is a convenience
rather than a safety feature.

## Consequences

### Good

- **Over-rolling is structurally impossible**, not merely unlikely. The only remaining path to it is
  the human ignoring a Latch, which requires a mouse click in another window.
- **A run of failures costs zero orbs.** In the spike each unreadable Read burned one; the accident
  that guard exists for — the orb leaving the cursor — now spends nothing while it is detected.
- **The whole cycle is testable on the development machine.** The 47 captures from our own code and
  the 113 from the AutoHotkey era become event sequences, not just parser inputs.
- **The app no longer depends on a client display setting** for its safety-critical computation.
- **The trigger key is left alone when idle**, so poe-graft running in the background does not break
  typing `[` anywhere else on the desktop.

### Bad, and accepted

- **Dead presses.** Every Unknown Verdict costs one press that does not Roll, and the first press of
  every session never Rolls. Under 1 in 1000 rolls will be a genuinely identical reroll misread as
  stale. This is the price and it is worth it.
- **Two states behave identically** (`Sighting`, `Resyncing`), which will look redundant to anyone
  reading the state machine without reading the messages it drives.
- **More code than the spike.** A pure state machine plus an executor is more moving parts than
  atomics and inline sleeps, in exchange for testability.
- **Numeric tier derivation depends on `data/ghastly-eye-jewel.json` being right.** The annotation
  cross-check is logged precisely so a wrong row is discoverable, but it will be discovered in a log
  file rather than by the app refusing to run.
- **[#18](https://github.com/Furizaa/poe-graft/issues/18) is now load-bearing UI copy.** Because
  arming is a mouse click in our own window and the hook is deaf while that window has focus,
  `Sighting` must tell the human to click into Path of Exile before pressing. A deferred curiosity has
  become a sentence the app has to say correctly.

## Alternatives considered

### Roll anyway on an Unknown Verdict — rejected

The spike's behaviour, and proven on device. Rejected because each such press carries a ~1-in-278
chance of destroying the Hit it just made, and the app's entire reason to exist is that specific
event. The measured Unknown rate is near zero at 130 ms, which makes the *cost* of the safe choice
near zero too — but it is the rate under good conditions, and the guard is for bad ones.

### The app re-copies itself until it has a Verdict — rejected

Fastest for the human: no dead presses at all. Rejected because it breaks the one-action-per-press
rule, which is the project's whole TOS defensibility
([#14](https://github.com/Furizaa/poe-graft/issues/14),
[#15](https://github.com/Furizaa/poe-graft/issues/15)). A second `Ctrl+C` with no press behind it is
an app-initiated invocation, and that is the line the paid tool crosses.

### Halt on the first Unknown Verdict — rejected

The strictest reading of fail-closed. Unnecessary once a Resync exists: the first Unknown is already
handled safely by a press that spends nothing, so halting adds interruption without adding safety.

### Trust the game's tier annotation — rejected

Smallest possible parser, and what ADR 0001 assumed (`generation == "prefix" && tier <= N`). Rejected
on the owner's call for the multi-mod future, and it turned out to be fragile for two further reasons:
the annotation is a client setting, and its mod name varies per tier.

### Validate rolled numbers *and* require annotation agreement — rejected for now

Would catch an off-by-one tier table immediately. Rejected because it makes a wrong row in RePoE's
data a hard stop for the human mid-craft, and because logging the disagreement gets the same
information without the interruption.

### A dedicated Latch-release hotkey — rejected

Fastest acknowledgement, no mouse. Rejected because a reflex press could discard a Hit, and
[#18](https://github.com/Furizaa/poe-graft/issues/18) makes it dead whenever our window is focused —
which is exactly when the human is looking at the Latch.

### Mouse wheel as the Trigger — rejected for v1

The map's original "click faster" idea, repointed at the trigger. Mouse events cannot be suppressed
([#13](https://github.com/Furizaa/poe-graft/issues/13)), so the scroll would also reach the game, and
scrolling risks nudging the cursor off the Anchor's tolerance.

### Keeping the state machine in `win32` — rejected

Proven code, less to write. Rejected because it is testable only on the machine that cannot run tests.

## Not decided here

- **What invalidates an Anchor** beyond the 24 px tolerance — window moves, resolution changes, the
  inventory panel closing. Still fog on the map.
- **Which sounds, and the window's layout** —
  [#9](https://github.com/Furizaa/poe-graft/issues/9).
- **Whether the Trigger Key is user-configurable**, and by what mechanism. A typed key-code fallback
  is required either way because [#18](https://github.com/Furizaa/poe-graft/issues/18) blocks
  learn-by-press while our window has focus.
- **Multi-mod targets** and what a Hit means when one of two lands. The hit test is shaped so this
  stays open; v1 needs one mod.
- **Where configuration is stored.** The app stores nothing today.
