# Spike session checklist — issue #17

Ordered script for the one session that has to happen at the gaming PC, in Path of Exile. It
settles the five questions in
[Spike on device: verify Shift-persist, Ctrl+C under Shift, and the trigger hook](https://github.com/Furizaa/poe-graft/issues/17),
which is what unblocks
[Design the roll cycle and the hit latch](https://github.com/Furizaa/poe-graft/issues/7).

Read it through once before starting. Steps 1–4 cost nothing; only step 7 spends currency.

## What to have ready

| | |
| --- | --- |
| **A Ghastly Eye Jewel, ilvl ≥ 83** | Below 83 the target tier cannot spawn at all. Any magic jewel works for the spike — it does not need to be the one you intend to keep |
| **Orbs of Alteration in your inventory** | **Not** a stash tab. Apply-repeat draws from the inventory; that is the whole reason the mechanism works this way |
| **The jewel in your inventory too** | Beside the orbs |
| **poe-graft on the second monitor** | Path of Exile windowed or borderless, so both are visible at once |

Budget: about 60 alterations. The spike does **not** need a tier-1 hit — it needs a sustained
*cycle*. Do not keep going hoping for one.

## 1 · Turn Sticky Keys off — first, not last

Sticky Keys changes what holding Shift means, silently and with no error, which would make every
other answer this session produces untrustworthy.

The app now reads the setting itself, so you do not have to remember: the **Environment** row at
the top of the spike panel shows `Sticky Keys ON` in red if it is on. It also flags whether the
*five-taps-on-Shift shortcut* is enabled — worth disabling too, since a Shift-heavy crafting
session is exactly the thing that trips it by accident.

Both live in **Settings → Accessibility → Keyboard**.

Do not continue while that badge is red.

## 2 · Get the build

Open poe-graft and press **Check for updates**. Confirm the header shows the run number of the
build that carries the spike panel — if there is no *Spike — issue #17* section, you are on an
older build.

## 3 · Install the hook

**Spike → 2 · Keyboard hook → Install hook.** The badge should read
`WH_KEYBOARD_LL installed`. If it fails, the raw error is on screen and in the log; stop and
report it.

This is the first time this app has ever installed a global keyboard hook, so it is also the first
chance Defender's behavioural engine has to object. A folder exclusion is already in place. If the
app dies or vanishes here, that is the interesting finding, not a setback.

## 4 · Choose the trigger key

Your keyboard has no F-row, so the F13–F24 keys Path of Exile ignores are not available. The key is
learned rather than guessed.

1. **Learn a key**, then press the key you want to use. Pick something Path of Exile has no
   binding for and that you will not hit by accident — `Scroll Lock`, `Pause`, `Insert`, a numpad
   key, `` ` ``.
2. Press **Use \<key\>**. The `trigger` badge should now name it.

Key codes are only recorded while *Learn a key* is on.

## 5 · Set the bounds for a first, cheap run

| Field | First run | Why |
| --- | --- | --- |
| Settle delay | `40` ms | Between the click and `Ctrl+C`, so the copy reads the *new* roll |
| Read timeout | `500` ms | AutoHotkey measured 15–32 ms, so this is very generous |
| Drift tolerance | `24` px | How far the cursor may wander off the jewel before the app refuses |
| **Roll cap** | **`5`** | Deliberately tiny. Prove the cycle works before spending real currency |

Leave **Suppress the trigger key** *off* for now, **Require Path of Exile in the foreground** *on*,
and **Copy mode B** *off*.

## 6 · Set up in game

1. Path of Exile in the foreground, inventory open.
2. **Right-click an Orb of Alteration** so it sits on the cursor.
3. **Hold Shift.** Keep holding it — this is what makes apply mode repeat.
4. Hover the jewel. Do not click.

## 7 · Arm and roll

Press **Arm** in the app (you can do this before moving to the game — arming forgets any previous
position, so the first press always recaptures it).

Then, in game, with Shift held and the jewel hovered:

- **First trigger press** — captures the jewel's position and injects *nothing*. The panel shows
  `item at x,y`.
- **Every press after that** — one roll: one injected click, one `Ctrl+C`, one read.

Press it five times. Watch the **Last roll** line. What you want to see:

- `OK`, a `copy` figure in the tens of milliseconds, and a few hundred `chars`.
- The mod summary **changing** between rolls.
- `shift down`.

Then stop and check the log panel: five `=== roll N: OK … ===` blocks, each followed by the full
item text.

If that all holds, raise the **Roll cap** to `50`, press **Arm** again, and do a sustained run.

### If something looks wrong

| What you see | What it means |
| --- | --- |
| `IDENTICAL to the previous roll`, repeatedly | The settle delay is too short — the copy is reading the item as it was *before* the click. Raise it to 80, then 120 |
| `TIMED OUT`, repeatedly | Nothing is being copied. Usually the jewel is no longer hovered. The app disarms itself after three in a row |
| **`spike DISARMED ITSELF`** | Three unreadable reads. Most likely the orb left the cursor and a click picked the jewel *up*. Check the cursor, re-arm, and note it |
| `refused — Shift is not held` | You let go. The app refuses rather than clicking, because a plain click would pick the jewel up |
| `refused — cursor … px off the captured` | You drifted off the jewel. Re-hover, or press **Forget position** to recapture |
| `injected 0 of 2 events` | UIPI — the game is running elevated and we are not. Worth knowing; report it |

Every refusal is a line in the log explaining itself. Presses that did not become rolls are
counted separately in the `presses` badge — a gap there is the fail-closed rule working, not a bug.

## 8 · What answers each question

Four of the five answer themselves in the log. Only the first needs your eyes.

1. **Does Shift-persist survive a sustained cycle of injected clicks?** — **You have to watch
   this one.** Keep an eye on whether the orb stays on the cursor and each click keeps applying.
   Note the roll number if apply mode ever drops out, using the **Note what you saw** box so it
   lands in the log in order. The machine's half of the answer is the run of `OK` reads with
   changing text.
2. **Can `Ctrl+C` be sent without releasing Shift?** — answered by step 7 succeeding at all. Mode A
   holds Shift right through the copy, which is exactly what AutoHotkey's `Send` may have been
   quietly undoing on our behalf. `OK` reads with `shift down` means yes.
3. **Is Sticky Keys off?** — recorded automatically in the log every time you arm.
4. **Read latency and timeout rate from our own code** — the `copy` figure on every roll, against
   AutoHotkey's 15–32 ms and ~1.8%.
5. **Does suppression reach Path of Exile?** — step 9.

## 9 · The suppression sub-test — no currency needed

Do this after the rolling, with the spike **disarmed** (suppression works independently of
arming, so nothing will be injected).

1. **Disarm.**
2. In Path of Exile, **Options → Input**, temporarily bind your trigger key to something visible
   and harmless — *Open Inventory* is ideal.
3. Suppression **off**: press the key. The inventory should toggle.
4. Suppression **on**: press it again. If the inventory does **not** toggle, keyboard suppression
   reaches the client — the asymmetry against the mouse that
   [#13](https://github.com/Furizaa/poe-graft/issues/13) found, now confirmed rather than assumed.
   If it *does* still toggle, keyboard suppression does not reach it either, and the trigger key
   has to be one the game genuinely ignores.
5. **Remove that binding again.**

## 10 · What to send back

The log has everything, including the full text of every roll — those are the fixtures the parser
will be tested against, and this is the only machine that can produce them.

**Log → Open folder**, then attach `poe-graft.log` to a comment on
[#17](https://github.com/Furizaa/poe-graft/issues/17).

Path, for reference: `%LOCALAPPDATA%\com.furizaa.poegraft\logs\poe-graft.log`

## What the app will not do, and what it cannot protect you from

It obeys the same rule the real app will: **exactly one injected left-click and one `Ctrl+C` per
trigger press you physically make.** No timer, no repeat without a fresh press, and nothing done in
reaction to what it read. Auto-repeat from holding the key down is one press, not a stream. Its own
injected input is ignored, so it cannot react to itself.

It refuses rather than guesses: no Shift held, cursor drifted, Path of Exile not in the foreground,
roll cap reached, a cycle still in flight — all refusals, all logged.

**What it cannot see** is whether an orb is still on your cursor. If apply mode drops out while you
are still holding Shift, the next click picks the jewel up instead of rolling it. That is why the
first run is capped at five, and why three unreadable reads in a row make it stop by itself.
