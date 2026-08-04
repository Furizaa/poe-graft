# Where GGG's line actually sits on macros and injected input

Research for [issue #14](https://github.com/Furizaa/poe-graft/issues/14). Compliance research only —
this document deliberately contains nothing about detection or enforcement mechanics. All sources
retrieved 2026-08-04.

## Evidence classes used below

Every claim in this document is tagged with one of:

- **(A) Written GGG rule** — GGG's own published Terms of Use or Developer Docs.
- **(B) Named GGG staff statement** — a forum post by an account with the `_GGG` suffix (or `Chris`,
  the founder's staff account), with a date.
- **(C) Community belief, no primary source** — widely repeated, traceable to no GGG statement.
- **(D) Inference from tolerance** — what appears to go unpunished. Evidence about tolerance, never
  about permission.

---

# Verdict

## (a) What the rules verifiably say

**"One action per keypress" is not folklore. It is GGG's written, published rule, and it has been
restated by named GGG staff continuously from 2013 to 2024.** The ticket's premise that this might be
community invention is wrong, and that is the single most important correction in this document.

The written rule lives in GGG's **Developer Docs**, under the sidebar heading *Third-Party Policy*
([pathofexile.com/developer/docs/index](https://www.pathofexile.com/developer/docs/index)) — **(A)**:

> Automation of key-strokes (or other inputs) that affect the game in any way **must** follow our
> macro rules:
>
> - Macros must be invoked manually by the user (automated invocations such as but not limited to:
>   timers, reacting to file changes, or from reading the screen are not allowed).
> - Each macro invocation must have one set function (it cannot cycle between actions).
> - The resulting function must only perform one action that interacts with the game (sending a
>   single chat message or command counts as one action).
>
> Introducing features against these guidelines or our Terms of Use can result in account termination
> for you **as well as** your users.

This exact text has been live and byte-identical since at least **2021-03-29** (earliest Wayback
snapshot of the page) through **2026-07-02**, verified against the live page on 2026-08-04.

Three of those clauses land directly on this project:

1. **"reacting to ... reading the screen are not allowed"** — an app that decides to act based on
   what it read from the item is an *automated invocation* by GGG's own definition. This is written,
   not inferred.
2. **"reacting to file changes ... not allowed"** — kills any client-log-triggered variant too.
3. **"must only perform one action that interacts with the game"** — caps a single manual trigger at
   one game-affecting action.

The **Terms of Use** ([link](https://www.pathofexile.com/legal/terms-of-use-and-privacy-policy),
clause 7 *Restrictions*) is the harder legal instrument but is far *less* specific — **(A)**:

> Under no circumstances, without the prior written approval of Grinding Gear Games, may you: [...]
>
> (b) Modify or adapt (including through third parties and third-party tools) the game client or its
> data, other than in the normal course of PoE gameplay as permitted in accordance with the Licence.
>
> (c) Utilise any automated software or 'bots' in relation to your access or use of the Website,
> Materials or Services.
>
> (e) Connect to the Servers through any software other than the authorised game client software.
>
> (i) Reverse engineer, de-compile or disassemble the Website, Materials or Services or seek to
> establish the technical processes, operations and communication protocols of the Website, Materials
> or Services through any means [...]

**The word "macro" does not appear anywhere in the Terms of Use.** Neither does "keystroke",
"input", or any cognate. I searched the full retrieved document. The only operative ToU hook for a
tool like this is **7(c)**, "automated software or 'bots'" — a phrase the ToU never defines. The
Developer Docs are what supply the definition.

**Named staff, spanning eleven years, all saying the same thing — (B):**

| Date | Staff | Thread | Operative quote |
|---|---|---|---|
| 2013-07-24 | **Chris** (Lead Developer) | [473902](https://www.pathofexile.com/forum/view-thread/473902/page/5#p4197749) — *"AutoHotKey list of macros for PoE"* | "Any macro that performs more than one action is bannable, as is anything that sends it based on a timer. It's fine to have a button that causes /oos, or /remaining or casts an aura, but having a timer to repeat actions or a macro to cast all your auras is not allowed. **This is because these are things that normal players cannot do, so they give advantages in terms of playing speed.**" |
| 2014-01-01 | Yeran_GGG | [734784](https://www.pathofexile.com/forum/view-thread/734784/page/1#p6359805) | "Any macro that performs more than one action is bannable, as is anything that sends it based on a timer." |
| 2014-05-23 | Yeran_GGG | [931994](https://www.pathofexile.com/forum/view-thread/931994/page/1#p7906080) | "Any macros that perform multiple actions **on the client** is against our Terms of Use." |
| 2015-12-18 | Jared_GGG | [1518992](https://www.pathofexile.com/forum/view-thread/1518992/page/1#p12247699) | "automated programs like this are against the terms of use and will result in action being taken towards your account." |
| 2016-06-30 | Rob_GGG | [1694310](https://www.pathofexile.com/forum/view-thread/1694310) | "We do not allow macros that perform more then one **server side** action." |
| **2016-10-31** | **Brian_GGG** | **[1762033](https://www.pathofexile.com/forum/view-thread/1762033/page/1) — *"Macro to spam currency ?"*** | **"we only allow macros which perform a single action and don't repeat/act on a time. I'm afraid that an 'auto clicker' would be breaching a few of those restrictions, and so not something which would be allowed."** |
| 2021-04-23 | Nichelle_GGG | [3092943](https://www.pathofexile.com/forum/view-thread/3092943) | "We're not currently banning for the use of macros in general, so long as they are **manually controlled and performing one 'action' per use**. It's bannable if it performs more than one action, or if it's on a timer (as this qualifies as \"botting\")." |
| **2023-04-23** | **Will_GGG** | **[3378076](https://www.pathofexile.com/forum/view-thread/3378076) — *"So orb chance and Macro's"*** | **"We recommend refraining from creating or using any program that automates gameplay or does more than one action with a keystroke or mouse click as well as anything that interacts with the game client to provide an advantage over other players or provide information that isn't normally visible."** |
| 2024-11-03 | Sian_GGG | [3584808](https://www.pathofexile.com/forum/view-thread/3584808) | "we cannot comment on the legality of third-party tools [...] I would recommend refraining from creating or using any programs that **automates or does more than one action with a keystroke or mouse click** [...]" |

The two bolded rows are on this exact use case: a macro to spam currency, and orb-spam crafting
macros. Both got the same answer.

**On AutoHotkey specifically — (B).** GGG has never banned AHK, and has never said AHK is
"approved". The relevant fact is that **Chris posted policy guidance into a thread literally titled
"AutoHotKey list of macros for PoE" and let the thread stand**, asking only that rule-breaking macros
be edited out. Sarno's community guide states the position GGG has consistently implied — **(C)** for
the framing, **(B)** for the substance:

> Does it matter if I am using AHK, proprietary software, etc? **No.** [...] in the eyes of the
> developers they are creating rules based on what a player may or may not do — not specifically how
> a player may go about doing so. Using AutoHotKey, or software which came with your mouse or
> keyboard, or even a physical solution such as a popsicle stick would all be viewed the same way.
> — [Sarno#0493, thread 2077975](https://www.pathofexile.com/forum/view-thread/2077975), 2018-01-22

So: **the mechanism is irrelevant to GGG. `SendInput` vs AHK vs a hardware macro mouse vs a physical
stick across the flask keys are all judged identically.** This is why the map's chart-time rule
("may synthesize Ctrl+C, never the click") does not map onto GGG's rules at all — GGG does not
distinguish keystrokes from clicks, and does not care who physically generated the event. It counts
**actions per manual invocation**.

**Input simulation vs automation — (A), and it is decided.** There is no GGG text using the phrase
"input simulation" or anything like it; I searched for it and found nothing. But GGG *has* drawn
exactly the distinction this project cares about, in different words. The written rule's two halves
are:

- **who decides when** — "Macros must be invoked manually by the user"; automated invocation
  including "reacting to ... reading the screen" is prohibited.
- **how much per decision** — "must only perform one action that interacts with the game".

That is precisely the input-simulation/automation line. **Synthesizing an action the human just asked
for, one action per manual trigger, is inside the written rule. A program that decides when to act —
including deciding from what it read — is outside it.** GGG has never said the synthesizing itself is
the problem.

## (b) What is folklore with no primary source

**1. "Moving the cursor then clicking constitutes botting."** This is the claim most load-bearing for
the demonstrated tool's design, and its *only* source is a private, unpublished email. Sarno's
guide — the most careful community compilation, which cites a public GGG post for nearly every other
claim — says of this one:

> Can a macro move the mouse cursor, then click on something? **No.**
> Sources; (Direct) I e-mailed GGG about this and was told that it would "definitely constitute
> botting".

There is no forum post, support article, or Developer Docs clause behind this. It may well be
accurate, but **it is unverifiable — (C)**. Note the guide's author flagged that same weakness
himself, and a reply in the thread ([Abdiel_Kavash, 2018-01-23](https://www.pathofexile.com/forum/view-thread/2077975))
pointed out the unresolved case:

> Assume inventory is already open -> move mouse to where portal scroll is, click (one server-side
> action) - I don't know if this is allowed. Seems to fit the letter of the law, not sure about the
> spirit.

Eight years later that question still has no public GGG answer.

**2. That "action" definitively means "server-side action".** This is the community's confident
framing, and it is what lets people argue that a Ctrl+C hover-copy is free. **The public record is
contradictory.** Rob_GGG (2016) said "server side action". Yeran_GGG (2014) said "multiple actions
**on the client**". The Developer Docs say "one action that interacts with the game" — which is
neither, and reads broader than server-side. **So whether an injected `Ctrl+C` counts against the
one-action budget is genuinely unresolved in GGG's public record — (C).** Do not design as though it
is settled. This is the sharpest ambiguity affecting poe-graft.

**3. That any tool is "approved" or "safe".** GGG has explicitly refused this. Sian_GGG,
2024-11-03 — **(B)**:

> Unfortunately, we cannot comment on the legality of third-party tools, as we aren't able to
> thoroughly and accurately check exactly how they work. We do not encourage the creation or use of
> third-party tools because they provide advantages for players that use them. I'm afraid that we're
> **unable to guarantee if a tool is allowed or would remain allowed in the future.**

There is no whitelist. There has never been a whitelist. "Awakened PoE Trade is fine, therefore X is
fine" is not an argument GGG has ever endorsed.

**4. That GGG has published documented enforcement cases for crafting macros.** They have not, and
they say they will not. ShaunB_GGG, 2024-09-11 — **(B)**: *"We are unable to discuss any moderation
action or bans on the forums."* The ban-reason string players report on the forums is the
undifferentiated **"Third Party Software"** (e.g. threads
[3247788](https://www.pathofexile.com/forum/view-thread/3247788),
[3255738](https://www.pathofexile.com/forum/view-thread/3255738), Feb–Mar 2022), and appeals are
handled privately with no published outcome. **Any claim about what specifically gets punished, in
either direction, is (C) or (D). There is no primary-source enforcement record to reason from.**

**5. That GGG will answer if you ask about alteration spam.** They were asked, in the clearest
possible terms, and declined. Thread
[3816549](https://www.pathofexile.com/forum/view-thread/3816549), *"Autohotkey for Alterations Spam.
Possible permaban?"*, 2025-07-21 — a player describing hand pain after 10,000+ alterations asked
directly. **Ayelen_GGG**'s entire public reply — **(B)**:

> Hi! Could you please contact us at support@grindinggear.com and we'll be able to provide you with
> some additional information.

GGG will not put an answer to this question in public. That is itself the finding.

## (c) The tool being replicated

Identified: **PoEconomics "EconCrafter"**, demo video *"AutoCrafting v.2 | POE2 & POE1"*, channel
**PoEconomics**, published **2026-06-21**
([youtube.com/watch?v=sH_lz_yNwPI](https://www.youtube.com/watch?v=sH_lz_yNwPI)). €16.99/month or €99
lifetime, sold at [poeconomics.com](https://poeconomics.com/). The YouTube page resisted normal
fetching; title, channel, upload date and description were recovered from the page's embedded
metadata rather than rendered content, and I could not watch the video itself.

**The author makes no safety claim. He makes the opposite claim.** Verbatim from the site's FAQ:

> **Can I get banned for using this tool?**
> **Yes, you can.**
> Like any third-party tool that is not officially developed, endorsed, or supported by Grinding Gear
> Games (GGG), using this tool in Path of Exile 1 or Path of Exile 2 carries a risk of account
> action, including suspension or permanent bans.
> In practice, EconCrafter works with surface-level data that is already visible in the game client.
> It does not modify game files in any way. Functionally it is an overcomplicated macro — not a
> memory editor, inject, or file-altering cheat.
> Under the hood it uses the same kind of Ctrl+C item copy that many known item pricers use [...]
> Instead of sending that text to a market lookup, EconCrafter reads what was copied and runs the
> macro action you defined earlier for that result [...]

And its own marketing copy describes precisely the behaviour the Developer Docs prohibit:

> "Set your crafting target once. Let the Crafter finish the rest."
> "The crafter automatically stops when your target hits [...] You can also run an infinite amount of
> bases in succession automatically with multibase."

"Reads what was copied and runs the macro action you defined earlier for that result" is *reacting to
reading the screen* — the exact phrase the Developer Docs use to define a prohibited automated
invocation. **This tool is not a model to copy for compliance purposes. Its own author does not
claim it is compliant.**

**GGG has never responded to it publicly** — I found no GGG statement referencing PoEconomics or
EconCrafter. **That is (D), not (A).** A tool being sold openly, with a payment processor and a
Discord, is evidence that GGG has not moved against it *yet*. It is not evidence that its design is
permitted, and it cannot be. GGG's published position is that they cannot audit third-party tools and
will not guarantee any of them; silence is exactly what that position predicts, whether the tool is
compliant or not.

## (d) Minimum injected actions per roll

**One injected left-click per roll, with the cursor stationary — but only if the alterations are in the
inventory and `Shift` is held.** Full evidence in §2.1–2.2.

Apply mode does **not** persist on its own, which is a correction to the ticket's framing. Right-click
arms the currency; a plain left-click applies one orb and disarms. **Holding `Shift` is what makes it
persist**, and with Shift held the cursor never has to move and the currency is never re-armed.

- **Inventory + Shift held:** **1 injected click** per roll (+1 `Ctrl+C` if reading by clipboard).
  `Shift` is a per-session modifier, not a per-roll action.
- **Currency in a stash tab:** **2 clicks + 3 cursor moves** per roll, because a 2018 bug report
  documents that Shift-repeat *always draws from inventory*, so a stash-sourced arm cannot persist.
  **This is why the demonstrated tool moves the cursor** — confirmed by reading a second open-source
  tool that is stash-based and contains no reference to Shift at all (§2.2).

So the minimum is **one game-affecting injected action per roll**, which is exactly the shape GGG's
written rule permits — *provided* the human supplies one manual trigger per roll, and provided the
`Ctrl+C` question in §1.4 resolves favourably.

## (e) Which keys PoE ignores

**`Num +`, `Num -`, `Num *`, `Num /` cannot be bound in Path of Exile 1** — a bug reported since 2011,
re-reported in 2022, never fixed. A trigger on one of these needs **no suppression at all**, which
sidesteps the entire keyboard-hook question. §2.4.

**F13–F24 are unverified** — one low-quality secondary claim, nothing official. Probe item, not a
finding.

**Whether `WH_KEYBOARD_LL` suppression reaches PoE is unknown**, and issue #13's mouse result is a
standing warning against assuming symmetry. Do not design anything that needs the answer. §2.4.

---

# Part 1 — detailed policy findings

## 1.1 Scope caveat on the Developer Docs — read this

The macro rules live inside GGG's **API documentation**, and the page's preamble scopes itself:

> This documentation includes requirements that form our API Policies. **Applications that interface
> with our APIs must adhere to these**, as well any requirements in our Terms of Service and Privacy
> Policy.

poe-graft does not intend to touch GGG's APIs at all. So a lawyer could argue the macro rules do not
bind it by the document's own scoping sentence, leaving only ToU 7(b)/(c).

**Do not lean on that.** Two reasons:

1. The macro-rules sentence itself is unqualified as to API use: *"Automation of key-strokes (or
   other inputs) that affect the game **in any way** must follow our macro rules."* It is not written
   as an API condition; it is written as a condition on affecting the game.
2. The macro rules are the *only* place GGG has written down what 7(c) means. The named-staff posts
   from 2013–2024 say the same thing in Help-and-Information threads that have nothing to do with the
   API. Treating the Developer Docs text as GGG's general position is the reading the staff record
   supports.

The honest summary: **the macro rules are GGG's best-documented statement of its position, they are
consistent with eleven years of staff answers, and they are not formally part of the contract you
agreed to.** The contract is thinner and vaguer; the documentation is specific.

## 1.1b Where the rules are *not* — checked, so nobody re-checks

- **There is no GGG support knowledge base.** `pathofexile.com/support` is a contact form, not an
  article index. GGG support is email-only (`support@grindinggear.com`). **There is no support article
  on macros, automation, or third-party programs, because there are no support articles at all.**
- **The Code of Conduct** ([thread 1457463](https://www.pathofexile.com/forum/view-thread/1457463),
  posted by Support 2015-10-20, last edited 2019-06-05) covers forum and chat conduct only. It does
  not mention macros, automation, botting, third-party programs, or cheating.
- **The Terms of Use** contains no occurrence of "macro", "keystroke", "input", or "automation of"
  — only the undefined "automated software or 'bots'" in 7(c).

So the complete universe of GGG's written statements on this topic is: **ToU clause 7, and the macro
rules in the Developer Docs.** That is all of it. Everything else is staff forum posts.

## 1.2 The rationale GGG gave, which is the useful design constraint

Chris Wilson, 2013, explaining *why*:

> This is because these are things that normal players cannot do, so they give advantages in terms of
> **playing speed**.

That is the underlying test, and it is more useful than the letter of the rule for judging a design.
A mechanism that lets the human do what they were already doing — apply one orb per deliberate
physical input — does not confer speed a normal player cannot achieve. A mechanism that reaches
15,000 rolls/hour manifestly does. **~4.2 rolls/second is not a rate a human reaches, and no reading
of GGG's rules makes it acceptable.**

## 1.3 What the rules do *not* prohibit

Worth stating positively, because the map's chart-time rule was pessimistic in the wrong places:

- **Nothing forbids injecting a click rather than a keystroke.** GGG's rules are indifferent to which
  input type is synthesized (Sarno's popsicle-stick framing; consistent with all staff posts, none of
  which distinguish click from key). The map's "never synthesize the click" rule has no basis in
  GGG's published rules. It was a self-imposed constraint, not a GGG one.
- **Nothing forbids reading the item.** Reading the clipboard is not an action that interacts with the
  game. Reading the game's log files is *explicitly* blessed ("Reading the game's log files is okay
  as long as the user is aware of what you are doing with that data"). Reading is fine; **acting on
  what you read is what the rules prohibit.**
- **Nothing forbids showing the human information** — mod pools, tiers, probabilities, a hit alarm —
  provided it comes from data the player can already see. Note Will_GGG's and Sian_GGG's caveat about
  tools that "provide information that isn't normally visible"; item text the player is hovering is
  normally visible, so a tier readout of a hovered item sits on the safe side of that line.
- **Nothing forbids a 1:1 input remap.** One mouse-wheel notch → one left-click is one physical human
  input producing one action. This is the same shape as a macro key that fires one flask, which GGG
  has repeatedly said is fine.

**One caveat on "hold the key to keep rolling".** The Nichelle_GGG post (2021-04-23) is the closest
thing to an answer on a held/toggled input, and it is ambiguous. The player asked, verbatim:

> is it allowed to make a marco that holds my Cylone spin button? so i just need to activate it and
> it spinns until i turn it off again?

Nichelle's reply restated the general rule ("manually controlled and performing one 'action' per
use... bannable if it performs more than one action, or if it's on a timer") **without answering the
toggle question either way**. A design where the human *holds* a trigger key and the app emits a
stream of clicks is on the wrong side of "one action per use" on any plain reading, and is
indistinguishable from a timer if the app paces the stream itself. **A tap-per-roll design is the only
one clearly inside the rule.** OS key auto-repeat producing many `WM_KEYDOWN` events from one physical
press is the same problem wearing a hat: it is one physical press but many events, and GGG has said
nothing about it. Treat auto-repeat as something to explicitly filter out, not to exploit.

## 1.4 The unresolved question that matters most to poe-graft

**Does an injected `Ctrl+C` hover-copy consume the one-action budget?**

- If "action" means **server-side action** (Rob_GGG 2016), no — a hover-copy generates no server
  traffic. Then `trigger → Ctrl+C + read + conditional click` is one action per keypress and
  compliant.
- If "action" means **any input that interacts with the game** (the Developer Docs' own wording, and
  Yeran_GGG's "on the client" framing), then `Ctrl+C` + click is **two**, and only one of them is
  allowed per trigger.

One point in favour of the permissive reading: **hover-copy is a first-party client feature with an
in-game keybind, not a hack.** The "Advanced Mod Descriptions" modifier is user-configurable in the
game's own UI options (see e.g. [thread 2629014](https://www.pathofexile.com/forum/view-thread/2629014/page/1),
a bug report about rebinding that modifier to Ctrl). Synthesizing it is synthesizing something the
client is designed to do on a keypress. That is an argument, not a GGG ruling — **(C)**.

GGG's public record does not resolve this. **This is the single question worth emailing
support@grindinggear.com about**, and it is the question Ayelen_GGG's 2025 redirect implies they will
answer privately. Design so that the answer can change the implementation cheaply — e.g. keep the
read on a separate human input from the apply, or read from the client log rather than by injecting
`Ctrl+C` at all (log reading is explicitly permitted and involves *no* injected input, which sidesteps
the whole question).

---

# Part 2 — alteration-spam mechanics

**Evidence quality warning for this whole section.** GGG documents none of this. There is no official
manual, no wiki page I could reach (`poewiki.net` is behind an Anubis challenge and
`pathofexile.fandom.com` returned HTTP 402 from this environment), and no patch note. Everything below
rests on **(c) open-source tool code** and **(d) community forum reports**, which agree with each other
but have never been confirmed by GGG. **Every claim here should be re-verified by a human at the game
before code depends on it.**

## 2.1 Does apply mode persist? — Yes, but only while Shift is held

**This is the answer, and it is not the one the ticket guessed.** Apply mode does *not* persist on its
own. Right-clicking the currency arms it; a plain left-click applies one orb and **disarms**. Holding
**Shift** is what keeps the currency on the cursor across applications, so repeated left-clicks keep
rolling the same item.

**Direct code evidence — (c).** [`w31w4ng/AwakenedAlterationSpam`](https://github.com/w31w4ng/AwakenedAlterationSpam)
is a working Python alteration-spam script. Its loop is the cleanest possible demonstration:

```python
pyautogui.keyDown('shift')            # once, at session start
try:
    while running and (alts_used + augs_used) < orb_cap:
        pyautogui.click()             # <-- one click. no coordinates. no cursor movement.
        alts_used += 1
        pyautogui.hotkey('ctrl', 'c') # read the result
        raw_text = pyperclip.paste()
        if re.search(user_regex, raw_text, re.IGNORECASE):
            break
finally:
    pyautogui.keyUp('shift')          # once, at session end
```

Note what is *absent*: no `pyautogui.moveTo(...)`, no coordinates anywhere in the file, and no
right-click. `pyautogui.click()` clicks wherever the cursor already is. Its README states the mechanic
explicitly:

> - Right-click the Alteration Orb to enter orb-spam mode (the cursor changes)
> - Hover over the item you want to roll
> - Press `=` — the script takes over from here
>
> The script will: **Hold `Shift` automatically (you do not need to hold it yourself)** [...] Stop and
> release `Shift` when a match is found

And in its feature list: *"Holds `Shift` automatically so you stay in orb-spam mode."*

**Corroborating community evidence — (d).** Two independent official-forum threads, five years apart:

- [Thread 446983](https://www.pathofexile.com/forum/view-thread/446983), 2013-07-02, *"Using an item
  multiple times?"* — the accepted answer: *"shift is the key you are looking for, use the currency
  item as per usual, when mousing over the item to apply it, hold shift"*, clarified as *"You have to
  start to use the currency as normal then hold shift to apply multiple's of that currency type."*
- [Thread 679214](https://www.pathofexile.com/forum/view-thread/679214), 2013-12-05, *"Use multiple
  orbs without rightclicking each time?"* — initial answers guessed Ctrl and were corrected: *"It's
  (holding)SHIFT-Click(s) Not CTRL"*. Worth noting because **the Ctrl-vs-Shift confusion is itself
  evidence that this is undocumented folk knowledge**, even though the folk converged on Shift.

Neither thread has a GGG staff reply. Both are archived.

**Consequence for the design.** With the currency **in the inventory** and Shift held:

> **Minimum injected actions per roll: one left-click.** The cursor never moves. The currency is never
> re-armed. Shift is a per-session modifier, not a per-roll action.

Add **one `Ctrl+C`** per roll if the read is done by clipboard hover-copy, giving **two injected input
events per roll, of which exactly one affects the game state.** That maps precisely onto GGG's written
rule shape — see §1.4 for the unresolved question of whether the `Ctrl+C` counts.

## 2.2 Why does the demonstrated tool move the cursor, then?

**Best-supported answer: because it sources currency from a stash tab, and the Shift-persist path is
documented not to work from a stash tab.** Of the four candidates the ticket listed, this one has
actual evidence behind it.

[Bug report thread 2260861](https://www.pathofexile.com/forum/view-thread/2260861/page/1), filed
2018-12-09 by SittingNerd#7633 after patch 3.5, titled *"Holding 'Shift' for Repeated Currency Use
Always Uses Inventory"* — **(d)**:

> whenever I right-click an orb in my currency tab and then proceed to hold shift and left-click an
> item... any similar currency in my inventory will always be used first

So the Shift-repeat path is **bound to inventory**, not to the stack you armed from. A tool built
around a stash tab as the currency source cannot rely on Shift-persist and must re-arm from the stash
each cycle — which requires exactly the cursor round trip observed in the demo. No GGG reply; thread
archived with replies disabled. **Confidence: moderate.** The behavioural difference is documented;
that it is *this tool's* reason is inference.

**Secondary contributing explanation, also moderate confidence:** PoEconomics' own video chapter list
includes *"Arm Grid & Multi-Position Capture"*, *"Currency, Item Position & Multi-Base Crafting"* and
advertises running *"an infinite amount of bases in succession automatically with multibase"*. Rolling
a *grid* of different items inherently requires moving the cursor between them, regardless of how
apply mode behaves. Some of the observed movement is probably this, not the stash issue.

**Ruled less likely:** stack exhaustion (would not produce movement on *every* roll) and shift-click
semantics (Shift is the thing that makes persistence work, not a complication). **I could not watch the
video** — YouTube served only navigation chrome and I recovered title/channel/date/description from
embedded page metadata. So this section is reasoning from the tool's own written descriptions plus the
issue #13 observations, not from watching it.

### The second tool confirms it — (c), and this is strong

[`m4iraki/poe-crafting`](https://github.com/m4iraki/poe-crafting) is an AHK v2 alteration/chaos/regal
spam framework, and it is the **stash-based** design. Its entire per-application primitive is
`lib/Stash.ahk`, `CurrencyItem.Use()`:

```ahk
Use(targetPos) {
    if (this.Count > 0) {
        Util.MClick(this.position, "Right")   ; re-arm the currency, EVERY roll
        Util.MClick(targetPos,    "Left")     ; apply it
        this.Count--
        Sleep(Config.PingDelay + Config.FPSDelay)
        update := Core.GetItem(targetPos)     ; read
```

and `Util.MClick` is *move-then-click*:

```ahk
static MClick(target, button) {
    MouseMove(target.centerX, target.centerY, 0)
    Sleep(Config.FPSDelay)
    Click(button)
    Sleep(Config.PingDelay)
}
```

Three things make this decisive:

1. **It re-arms with a right-click on every single roll**, and moves the cursor to do it. Exactly the
   behaviour observed in the PoEconomics demo.
2. **`grep -rn "Shift\|shift"` across the whole repo returns zero hits.** This tool does not know about
   the Shift mechanic at all — or cannot use it.
3. **Its coordinates are stash coordinates.** `Currencies.Alteration := CurrencyType("Orb of
   Alteration", 87, 250)` and its siblings are the fixed slot positions of the **currency stash tab**;
   `_RawCraftItem := { x: 285, y: 370, w: 97, h: 182 }` is a stash slot, not an inventory slot. The
   whole workflow lives inside the stash.

So the two tools bracket the answer cleanly, and the bracket lines up with the 2018 bug report:

| | `AwakenedAlterationSpam` | `m4iraki/poe-crafting` |
|---|---|---|
| Currency source | **inventory** | **currency stash tab** |
| Uses Shift-persist | **yes**, held all session | **no** (absent from source) |
| Cursor movement per roll | **none** | 3 × `MouseMove` |
| Clicks per roll | **1** (left) | **2** (right to re-arm, left to apply) |
| Read | `Ctrl+C` | `Ctrl+Alt+C` |

**Conclusion, high confidence: cursor movement is a consequence of sourcing currency from the stash,
not an inherent requirement of alteration spam.** Put the alterations in the inventory, hold Shift, and
the cursor never has to move.

### Bonus: this settles a live contradiction on the map

`poe-crafting`'s read primitive is `Core.GetItemDetailedText`:

```ahk
static GetItemDetailedText(item) {
    A_Clipboard := ""                                  ; poison
    MouseMove(item.centerX, item.centerY, 0)
    Sleep(Config.FPSDelay)
    Send("^!c")                                        ; Ctrl+Alt+C
    if !ClipWait(0.5)
        return ""
    return A_Clipboard
}
```

Two findings the map wanted:

- **`Send("^!c")` is `Ctrl`+`Alt`+`C`.** A working tool that needs tier annotations uses the Alt
  variant. This is evidence for issue #4's position over issue #3's on the map's flagged contradiction
  (*"⚠️ Contradicts the clipboard ticket on whether Alt+Ctrl+C is required"*). **(c)**, still worth the
  on-device check, but the contradiction now has a thumb on one side of the scale. Note the consequence
  the map already anticipated: holding Alt pins an advanced tooltip on every roll.
- **The poison-then-`ClipWait` protocol issue #3 specified is exactly what this tool does** — clear the
  clipboard, send the copy, wait for it to become non-empty, with a 500 ms timeout and a 3-attempt
  retry in `Core.GetItem`. Independent confirmation of that design, from the wild.

## 2.3 What cancels apply mode

| Event | Answer | Evidence |
|---|---|---|
| Injected `Ctrl+C` | **Does not cancel.** | **(c)** — `AwakenedAlterationSpam` sends `Ctrl+C` between every click inside the Shift-held loop and keeps rolling. If `Ctrl+C` disarmed, the script could not work at all. Strong. |
| Releasing Shift | **Cancels the persistence** (the next click would be the last, or the currency drops). | **(c)** — the script's `finally: keyUp('shift')` is its deliberate teardown. |
| Right-click | **Almost certainly cancels / returns the currency.** | **(e)** inference — see §2.5. |
| Window losing focus | **Unknown.** | No evidence either way. Needs on-device probe. |
| Cursor leaving the inventory panel | **Unknown.** Likely tolerated (the currency stays on the cursor), but a click on an invalid target may disarm. | No evidence. Needs on-device probe. |
| Stack running out | **Certainly ends it** — nothing left to apply. | **(e)** trivially. Both the tools reviewed cap the session rather than handle this. |
| `Escape` | **Unknown.** Plausibly closes the panel and drops the currency. | No evidence. |

The three "unknown" rows are all cheap to test in one sitting and all matter for a fail-closed gate.

## 2.4 Which keys PoE ignores — a trigger key needs no suppression

**The best-evidenced answer: the numpad operator keys `Num +`, `Num -`, `Num *`, `Num /` cannot be
bound in Path of Exile 1 at all.** That is precisely what a trigger key wants — PoE will not act on
them, so nothing needs suppressing.

[Bug report 3295638](https://www.pathofexile.com/forum/view-thread/3295638), 2022-08-20, Draky#4017 —
**(d)**:

> Some numpad keys are not usable for keybinds while others are. The ones I have noticed are num +,
> num -, num * and num /

and the reporter notes *"this topic has been brought up since 2011"*. No GGG reply; archived
unresolved. **Four years of "it's still broken" is unusually good evidence that the game genuinely
does not see these keys.** Confidence: moderate-high, trivially confirmable on device in ten seconds.

**F13–F24: unverified, do not assume.** I found one secondary claim that PoE recognises F13–F24 when
entered manually, from a low-quality aggregator, and nothing on the official forums. If they *are*
bindable they are also *ignorable* (just leave them unbound), which is nearly as good — but "PoE
ignores them" is not established. **Treat F13–F24 as a probe item, not a finding.**

**Other numpad keys and unbound main-row keys:** bindable, therefore only free by convention. Note
that `AwakenedAlterationSpam` picks `=` and `-` as its start/stop hotkeys — main-row keys that are
unbound by default but perfectly bindable, so they are safe only for a user who has not bound them.

**Keyboard suppression via `WH_KEYBOARD_LL` — genuinely unknown, and do not assume symmetry with the
mouse result.** I found no reliable evidence either way. The AHK ecosystem's PoE scripts do rely on
default (blocking) hotkeys and `#InstallKeybdHook`, which suggests keyboard suppression *does* reach
PoE, but I could not find a clean primary report and the mouse probe in issue #13 is a standing warning
against reasoning by analogy: PoE reads mouse buttons via Raw Input, and if it also registers a Raw
Input device for the keyboard, low-level keyboard suppression would fail identically. **Evidence class:
none. This is a probe item.**

**The design consequence is that you should not need the answer.** If the trigger is a key PoE cannot
bind (numpad operators), suppression is moot. Designing around a key that needs no suppression removes
an entire class of soft-lock risk — which is the same lesson issue #13 already paid for once.

## 2.5 Right-click on a hit, and picking the item up

**Right-click while apply mode is active almost certainly cancels apply mode rather than picking the
item up.** In PoE 1, right-clicking with something held on the cursor is the standard "put it back"
gesture, and items are picked up onto the cursor with a *left*-click. So the demo's apparent
"right-click to pick the item up" is more parsimoniously read as **right-click to disarm the
currency** — which is exactly what a tool would do on a hit, and would look the same to an observer:
the roll stops and the cursor changes.

**Evidence class: (e) inference.** I could not find a primary source for right-click-while-armed
behaviour, and I could not watch the video. **This is a guess and should be labelled as one.** But note
that it does not matter much: whichever it is, the safe latch action is simply *don't inject the next
click*, which needs no counter-action at all. Neither reviewed tool right-clicks to stop —
`AwakenedAlterationSpam` just `break`s out of the loop and releases Shift.

Whether an item held on the cursor can still receive currency is likewise **unverified**. It is very
likely it cannot (the currency and the item would both need to be on the cursor), which is why
picking the item up *would* work as a hard stop if the app wanted one — but that is inference, and a
hard stop implemented by injecting *more* input is a worse design than one implemented by injecting
less.

## 2.5b What this means for which mechanism family the project picks

Not a decision for this ticket, but the research points somewhere specific and it would be dishonest to
bury it.

**The mechanism family that is inside GGG's written rules is: human taps a trigger key once per roll →
app injects exactly one Shift+left-click → app reads the result → if it is a hit, the app refuses the
*next* trigger.** One manual invocation, one game-affecting action, the program never deciding when to
act. The latch is a *refusal to act*, which needs no suppression and cannot soft-lock the mouse,
because the app is not touching the physical click path at all.

This is achievable at **one injected click per roll with a stationary cursor**, which is the whole point
of §2.1. It requires the alterations to be in the **inventory**, not a stash tab.

**The family that is outside the written rules is the one the paid tool uses**: the app drives the loop
and stops on a read. "Reacting to ... reading the screen" is the Developer Docs' own example of a
prohibited automated invocation, and 15,000 rolls/hour is not a rate a human reaches. Replicating that
design cannot be made compliant by tuning it — the defect is structural.

The honest cost: a tap-per-roll design does not give the human 15,000 rolls/hour. It gives them
whatever they can tap, with the over-roll problem solved. **That is what the acceptance test on the map
actually asks for** — *"the app stopped my clicks on the T1 hit"* — and it is a strictly better outcome
than the status quo without needing the app to do the rolling.

## 2.6 Crafting simulators — no UI-loop evidence

`doomeer/kalandralang` and `DanielWieder/PoeCraftLib` model crafting *outcomes* (mod pools,
probabilities, expected currency cost). They are useful for probability verification — which issue #4
already did via RePoE — and contain **no evidence about the input loop**, because they never touch the
game client. Nothing to extract for this question.

## 2.7 Note on how the folklore propagates

Worth recording, because it is the failure mode this ticket exists to avoid.
`AwakenedAlterationSpam`'s README states:

> GGG's [forum guidance on macros](https://www.pathofexile.com/forum/view-thread/2077975) further
> clarifies that scripts are only acceptable if they produce a single server-side action per keypress.

The link goes to **Sarno's player-written guide**, not to GGG. The substance happens to be roughly
right (see §1's staff table), but the citation is to a community document described as official, and
the "server-side" qualifier — the part that is *not* settled — is presented as GGG's wording. That is
precisely how "one action per keypress" came to be repeated as policy with the ambiguity sanded off.
The rule is real; the confident gloss on what "action" means is not.

To that repo's credit, its own conclusion is honest and matches mine:

> This script runs an automated loop with repeated clicks, which falls outside what GGG considers
> acceptable.

---

# Open questions needing on-device or off-repo confirmation

**Ask GGG (email `support@grindinggear.com`) — the only route they have said is open:**

1. **Does an injected `Ctrl+C` hover-copy count against the one-action-per-invocation budget?** This is
   the single question that decides whether a compliant poe-graft can read and apply on the same
   trigger. §1.4. GGG's public record is contradictory ("server side action" vs "on the client" vs "one
   action that interacts with the game"), and Ayelen_GGG's 2025 reply implies they will answer this in
   private.
2. **Is a 1:1 remap that synthesizes a click from a physical key/wheel input acceptable?** i.e. is the
   *type* of synthesized input irrelevant, as the popsicle-stick framing implies. No public GGG
   statement addresses synthesized *clicks* specifically.
3. **Is a held or toggled trigger acceptable, or must each application come from a discrete press?**
   §1.3. Nichelle_GGG was asked essentially this in 2021 and did not answer it.

**On-device probes at the game (cheap, all in one sitting):**

4. **Confirm Shift-persist**: right-click an alteration stack in the *inventory*, hold Shift, left-click
   a magic item repeatedly. Does it keep rolling without re-arming? Does the cursor need to stay
   still? — §2.1. Everything in this document's mechanics half depends on this.
5. **Confirm the stash-tab difference**: arm from a currency stash tab with *no* alterations in
   inventory, hold Shift, click. Does it apply at all? Does it persist? — §2.2. This decides whether
   the app can ever avoid cursor movement in the user's real workflow.
6. **Confirm `Num +` / `Num -` / `Num *` / `Num /` are unbindable** in the game's Input options — §2.4.
   Ten seconds. If true, pick one as the trigger and the whole suppression question disappears.
7. **Test F13–F24**: does the Input options screen accept them? Does the game respond to them when
   unbound? — §2.4.
8. **Does `Ctrl+C` while armed disarm?** Strongly implied not to, but confirm directly rather than
   inferring it from a third party's script working — §2.3.
9. **Does losing window focus, or moving the cursor out of the inventory panel, disarm?** — §2.3. Both
   are states the app will routinely cause (it is a window on a second monitor).
10. **Does `Escape` disarm, and does right-click disarm or pick up?** — §2.3, §2.5.
11. **Does the client generate one apply per click at high injected rates**, or does it coalesce/drop?
    Relevant only to whether a read can keep up, not to compliance.

**Unexplained in the source, worth checking:**

12. `AwakenedAlterationSpam` claims that **holding `Alt` while clicking applies an Augmentation Orb
    instead of an Alteration Orb** (`keyDown('alt'); click(); keyUp('alt')`, and the README's *"an
    Augmentation Orb (hold `Alt` + click)"*). **I can find no PoE 1 mechanic that does this** and no
    documentation of it. Either it is an undocumented feature, or the script is wrong and silently
    applies alterations on that path, or it depends on some setup the README omits. Do not build on it.
    Note also that `Alt` is the advanced-mod-description modifier, so holding it has a known unrelated
    effect on the tooltip.

**Could not be resolved from this environment:**

13. Reddit is unreachable here (`old.reddit.com` and the JSON API both refuse), so two Reddit citations
    in Sarno's guide are unverified. Neither is load-bearing.
14. `poewiki.net` (Anubis challenge) and `pathofexile.fandom.com` (HTTP 402) both refused. **No wiki
    page on currency application mechanics was read for this document.** If a human can open
    `poewiki.net/wiki/Currency` and `poewiki.net/wiki/Orb_of_Alteration`, that may either confirm or
    complicate §2.1–2.3 cheaply.
15. The YouTube demo's actual content. §2.2 reasons from the tool's marketing copy and issue #13's
    observations instead.

---

# Source index

**GGG written rules**

- Terms of Use, clause 7 *Restrictions* — https://www.pathofexile.com/legal/terms-of-use-and-privacy-policy
  (retrieved 2026-08-04; the Privacy Policy half is dated "Last Updated: October 2024"; the Terms of
  Use half carries **no date of its own**, only an "Amendments" clause reserving the right to change
  it. There is no version history.)
- Developer Docs, *Third-Party Policy* / macro rules — https://www.pathofexile.com/developer/docs/index
  (retrieved 2026-08-04; text verified identical in Wayback snapshots `20210329024243`,
  `20220118165224`, `20260702160734`)

**Named GGG staff posts** — all reachable via the `/filter-account-type/staff` view of each thread,
which is how they were retrieved:

- Chris, 2013-07-24 — https://www.pathofexile.com/forum/view-thread/473902/page/5#p4197749
- Yeran_GGG, 2014-01-01 — https://www.pathofexile.com/forum/view-thread/734784/page/1#p6359805
- Yeran_GGG, 2014-05-23 — https://www.pathofexile.com/forum/view-thread/931994/page/1#p7906080
- Mark_GGG, 2014-05-15 — https://www.pathofexile.com/forum/view-thread/924751#p7847910 (relevant only
  as a datapoint that GGG devs disclaim knowing macro rules: *"I don't know the exact legality rules
  for macros (not my area)"*)
- Jared_GGG, 2015-12-18 — https://www.pathofexile.com/forum/view-thread/1518992/page/1#p12247699
- Rob_GGG, 2016-06-30 — https://www.pathofexile.com/forum/view-thread/1694310
- Brian_GGG, 2016-10-31 — https://www.pathofexile.com/forum/view-thread/1762033/page/1
- Nichelle_GGG, 2021-04-23 — https://www.pathofexile.com/forum/view-thread/3092943
- Kane_GGG, 2020-11-04 — https://www.pathofexile.com/forum/view-thread/2984212 (declines to answer in
  public, moves to PM)
- Will_GGG, 2023-04-23 — https://www.pathofexile.com/forum/view-thread/3378076
- ShaunB_GGG, 2024-09-11 — https://www.pathofexile.com/forum/view-thread/3572697
- Sian_GGG, 2024-11-03 — https://www.pathofexile.com/forum/view-thread/3584808
- Ayelen_GGG, 2025-07-21 — https://www.pathofexile.com/forum/view-thread/3816549

**Community compilations (secondary)**

- Sarno#0493, *"[Guide] What a macro is allowed to do in Path of Exile"*, 2018-01-22, last edited
  2023-03-31 — https://www.pathofexile.com/forum/view-thread/2077975. The best community compilation;
  cites a public GGG source for nearly every claim, and is transparent where it cannot.
- Exiled Exchange 2 FAQ — https://kvan7.github.io/Exiled-Exchange-2/faq — *"There are no approved
  apps created by community. If app complies with the game ToS, does one server action per button
  press and doesn't interact with the game client itself [...] it can be considered safe."* This is a
  tool author's self-assessment, **(C)**, not a GGG position.

**Mechanics sources (community forum, no GGG reply on any of them)**

- Thread 446983, 2013-07-02, *"Using an item multiple times?"* — https://www.pathofexile.com/forum/view-thread/446983
- Thread 679214, 2013-12-05, *"Use multiple orbs without rightclicking each time?"* — https://www.pathofexile.com/forum/view-thread/679214
- Thread 2260861, 2018-12-09, *"Holding 'Shift' for Repeated Currency Use Always Uses Inventory"* — https://www.pathofexile.com/forum/view-thread/2260861/page/1
- Thread 3295638, 2022-08-20, *"Some numpad keys not usable for keybinds"* — https://www.pathofexile.com/forum/view-thread/3295638
- Thread 2629014, *"Advanced Mod Description UI option doesn't work if you change the key to Ctrl"* — https://www.pathofexile.com/forum/view-thread/2629014/page/1

**Open-source tool code read for this document**

- `w31w4ng/AwakenedAlterationSpam` — https://github.com/w31w4ng/AwakenedAlterationSpam — single-file
  Python; inventory-sourced, Shift-held, one click per roll, `Ctrl+C` read
- `m4iraki/poe-crafting` — https://github.com/m4iraki/poe-crafting — AHK v2 framework; stash-sourced,
  no Shift, right-click re-arm + left-click per roll, `Ctrl+Alt+C` read. Files that matter:
  `lib/Stash.ahk` (`CurrencyItem.Use`), `lib/Util.ahk` (`MClick`), `lib/Core.ahk`
  (`GetItemDetailedText`), `lib/AlterationCrafting.ahk` (`ExecuteLoop`)

**The replicated tool**

- Demo video — https://www.youtube.com/watch?v=sH_lz_yNwPI ("AutoCrafting v.2 | POE2 & POE1",
  PoEconomics, 2026-06-21)
- Product site and FAQ — https://poeconomics.com/

**Could not be retrieved in this session**

- Reddit (`old.reddit.com` and the JSON API both refused). Two Reddit threads Sarno cites as sources
  — the "popsicle stick" threads on flask automation, and a Chris comment permitting alternative
  mouse cursors — **could not be independently verified here.** Neither is load-bearing for this
  project's decision.
- The YouTube demo video's actual content. Metadata only.
