/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * A synthetic `CycleStatus` feed, so every variant can be seen in every state on a Mac.
 *
 * Why this is needed at all: `getCycleStatus()` on macOS returns the stub — `supported: false`, state
 * `Idle`, no rolls, no Verdict, no timing, and an **empty `targetGroup`**, which also makes
 * `set_target` reject. So the session half of the real window renders nothing and the setup half is
 * inert on the development machine, and the two questions #9 most needs answered — *what is
 * unmissable when the Hit lands* and *what does `Halted` look like* — cannot be judged at all.
 *
 * `UI.md` explicitly sanctions pointing a prototype at a stub. The alternative considered and not
 * taken was giving the `cfg(not(windows))` stub a real `CraftSession` driven by synthetic events,
 * which would make the whole window replayable from the Mac permanently. That is worth doing — but it
 * is production work with a permanent home, and it does not belong on a throwaway branch. Filed as a
 * note on #9 instead.
 *
 * **The message strings below are copied verbatim from `crates/core/src/cycle.rs`.** That is
 * deliberate and load-bearing: the owner's complaint is "walls of text", and most of the text is
 * Rust's. Paraphrasing it here would make every variant look better than the real thing and prove
 * nothing. Where a variant wants shorter copy, that is a proposed change to `crates/core` and is
 * called out as one.
 */
import type { CycleStatus, CycleState } from "../api";

/** The states a human can actually perceive. `Rolling` lasts ~130–210 ms and is left out on purpose. */
export const drivableStates: CycleState[] = [
  "Idle",
  "Sighting",
  "Ready",
  "Resyncing",
  "Latched",
  "Halted",
];

export type Knobs = {
  state: CycleState;
  targetGroup: string;
  tierThreshold: number;
  rolls: number;
  /** The tier the last Read derived, when there was one. */
  lastTier: number | null;
  consecutiveUnknown: number;
  /** Turn the preflight problems on, so the variants can be judged with something wrong. */
  stickyKeys: boolean;
  hookDeaf: boolean;
  shiftDown: boolean;
  inGame: boolean;
  /** Which of the three fixture captures the last Read landed on. */
  captureIndex: number;
};

export const defaultKnobs: Knobs = {
  state: "Ready",
  targetGroup: "MinionAddedPhysicalDamage",
  tierThreshold: 1,
  rolls: 26,
  lastTier: null,
  consecutiveUnknown: 0,
  stickyKeys: false,
  hookDeaf: false,
  shiftDown: true,
  inGame: true,
  captureIndex: 1,
};

const UNKNOWN_LIMIT = 3;

/**
 * Core's own sentence for each state, verbatim from `cycle.rs`.
 *
 * Read these as the brief rather than as filler. The `Halted` one is 55 words in a single paragraph
 * with a three-item checklist buried inside it — the clearest single instance of what the owner
 * called walls of text, and the strongest argument that some of #9's answer is a `crates/core` change
 * rather than CSS.
 */
const messageFor = (k: Knobs): string => {
  switch (k.state) {
    case "Idle":
      return "Idle. The Trigger Key is no longer suppressed.";
    case "Sighting":
      return (
        `Armed for ${k.targetGroup} at Tier ${k.tierThreshold} or better. Click into Path of Exile ` +
        "with Shift NOT held — a Shift-click while an orb is on the cursor would apply it to whatever " +
        "you clicked. Then hold Shift, hover the jewel, and tap the Trigger Key once: the first press " +
        "Reads the jewel and spends no Alteration."
      );
    case "Ready":
      return k.lastTier === null
        ? `Miss — ${k.targetGroup} is not on the jewel. Press to Roll.`
        : `Miss — ${k.targetGroup} rolled Tier ${k.lastTier}, worse than Tier ${k.tierThreshold}. Press to Roll.`;
    case "Rolling":
      return "Rolling…";
    case "Resyncing":
      return (
        `Unknown — poe-graft has lost track of the jewel (${Math.max(1, k.consecutiveUnknown)} of ` +
        `${UNKNOWN_LIMIT} before it stops). Press again to Resync: that press Reads without Rolling ` +
        "and spends no Alteration."
      );
    case "Latched":
      return (
        `HIT — ${k.targetGroup} at Tier ${k.lastTier ?? k.tierThreshold} after ${k.rolls} Roll(s). ` +
        "Latched: the next press will not Roll. Acknowledge with the mouse when you have taken the " +
        "jewel off the cursor."
      );
    case "Halted":
      return (
        `HALTED — ${UNKNOWN_LIMIT} Reads in a row came back with an Unknown Verdict. The app cannot ` +
        "see what state the game is in, so it has stopped rather than keep clicking. Check that the " +
        "jewel is still hovered, that an Orb of Alteration is still on the cursor, and that Shift is " +
        "still held. Re-arm to continue."
      );
  }
};

const haltReasonFor = () =>
  `${UNKNOWN_LIMIT} Reads in a row came back with an Unknown Verdict. The app cannot see what state ` +
  "the game is in, so it has stopped rather than keep clicking. Check that the jewel is still " +
  "hovered, that an Orb of Alteration is still on the cursor, and that Shift is still held.";

export const mockStatus = (k: Knobs): CycleStatus => {
  const armed = k.state !== "Idle";
  const verdict =
    k.state === "Latched"
      ? ("Hit" as const)
      : k.state === "Ready"
        ? ("Miss" as const)
        : k.state === "Resyncing"
          ? ("Unknown" as const)
          : null;

  return {
    // Still telling the truth about injection: nothing here can click. The chrome says "SIMULATED"
    // so a replay is never mistaken for a device run.
    supported: false,
    // The hook is installed at startup and stays installed — it is not a consequence of arming. Tying
    // this to `armed` made every variant's preflight fail while Idle, which is exactly when a preflight
    // is supposed to pass, and left variant B's Arm button permanently disabled.
    running: true,
    state: k.state,
    message: messageFor(k),
    targetGroup: k.targetGroup,
    tierThreshold: k.tierThreshold,
    rolls: k.rolls,
    anchor: armed && k.state !== "Sighting" ? [1284, 693] : null,
    pressesDropped: Math.floor(k.rolls / 9),
    consecutiveUnknown: k.state === "Resyncing" ? Math.max(1, k.consecutiveUnknown) : 0,
    unknownLimit: UNKNOWN_LIMIT,
    lastVerdict: verdict,
    lastTier: verdict === "Miss" || verdict === "Hit" ? k.lastTier : null,
    haltReason: k.state === "Halted" ? haltReasonFor() : null,
    suppress: armed,
    triggerVk: 219,
    triggerName: "[",
    keysSeen: k.hookDeaf ? 0 : 4120 + k.rolls * 3,
    presses: k.rolls + Math.floor(k.rolls / 9) + 1,
    shiftDown: k.shiftDown,
    foreground: k.inGame ? "PathOfExile_x64.exe" : "poe-graft.exe",
    hits: k.state === "Latched" ? 1 : 0,
    halts: k.state === "Halted" ? 1 : 0,
    blips: k.state === "Resyncing" ? Math.max(1, k.consecutiveUnknown) : 0,
    // `copy_ms` is Ctrl+C → the clipboard sequence number moved, and nothing more: the 1–8 ms number.
    // `cycle_ms` is the whole plan including the game's ~130 ms settle, which is what sets the rate.
    copyMs: armed && k.state !== "Sighting" ? 3 : null,
    cycleMs: armed && k.state !== "Sighting" ? 168 : null,
    accessibility: {
      stickyKeysOn: k.stickyKeys,
      stickyKeysAvailable: true,
      filterKeysOn: false,
      toggleKeysOn: false,
    },
  };
};

/**
 * A plausible log tail, one line per event, in core's own phrasing.
 *
 * Variant D is built on the claim that this — short lines, in order — is not what the owner meant by
 * walls of text, and is the thing the real window currently dumps into a `<pre>` at the very bottom
 * where nobody looks mid-craft.
 */
export const mockLog = (k: Knobs): string[] => {
  const lines: string[] = [
    `──── armed · target ${k.targetGroup} · Tier Threshold ${k.tierThreshold} · settle 130ms · read timeout 400ms · read settle 40ms · Anchor tolerance 6px · Unknown limit 3 ────`,
    `Anchor captured at 1284,693. Baseline Read: ${k.targetGroup} is not on the jewel.`,
  ];
  const tiers = [null, 6, 5, 4, 3, 6, 5, 4, 2, 6, 5];
  for (let roll = 1; roll <= Math.min(k.rolls, 9); roll++) {
    lines.push(`Roll ${roll} — one Alteration spent.`);
    const tier = tiers[roll % tiers.length];
    lines.push(
      tier === null
        ? `Miss — ${k.targetGroup} is not on the jewel. Press to Roll.`
        : `Miss — ${k.targetGroup} rolled Tier ${tier}, worse than Tier ${k.tierThreshold}. Press to Roll.`,
    );
  }
  if (k.rolls > 9) {
    lines.push(`… ${k.rolls - 9} further Rolls, all Miss …`);
    lines.push("Refused a Trigger Press: Shift is not held.");
    lines.push(`Roll ${k.rolls} — one Alteration spent.`);
  }
  if (k.state === "Resyncing") {
    lines.push(
      `Unknown Verdict (${Math.max(1, k.consecutiveUnknown)} in a row): the Read established nothing about the Target Mod.`,
    );
  }
  if (k.state === "Latched") {
    lines.push(
      `HIT — ${k.targetGroup} at Tier ${k.lastTier ?? k.tierThreshold} after ${k.rolls} Roll(s). Latched: the next press will not Roll.`,
    );
  }
  if (k.state === "Halted") {
    for (let n = 1; n <= 3; n++) {
      lines.push(`Unknown Verdict (${n} in a row): the game never copied.`);
    }
    lines.push(`HALTED — ${haltReasonFor()} Re-arm to continue.`);
  }
  return lines;
};
