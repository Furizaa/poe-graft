/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * # This file is a proposal to change `crates/core`, written in TypeScript so it can be judged.
 *
 * ADR 0001 is explicit that **the frontend holds no domain logic**, and `Craft.tsx` is explicit that
 * every explanatory sentence it renders is Rust's own `status.message`, verbatim, so that the window
 * and the log file cannot disagree. Both of those are right and #9 does not get to overturn them.
 *
 * But the owner's verdict on the shipped window is *"walls of text and debug stuff"*, and most of the
 * text is Rust's. The `Sighting` message is 58 words; the `Halted` message is 55 words with a
 * three-item checklist buried inside the prose. No amount of layout fixes a paragraph — so part of
 * #9's answer is necessarily a change to core's strings, and the only way to let the owner *judge*
 * that change is to render it.
 *
 * So everything here is duplicated domain copy, deliberately, in exactly one file, and **if any
 * variant using it wins, this file does not ship — its contents move into
 * `crates/core/src/cycle.rs`** so the log keeps getting the same words the window shows. The variants
 * that use it are labelled in the switcher; variant A deliberately does not, so the owner can see how
 * far pure layout gets on its own.
 *
 * The proposal itself has a shape worth stating: **one short imperative line, plus a list where core
 * currently buries one in prose.** Nothing is deleted — the long sentence stays available, and stays
 * the thing written to the log.
 */
import type { CycleStatus } from "../api";

/** The one thing to do next, in the fewest words that are still true. */
export const headline = (status: CycleStatus): string => {
  switch (status.state) {
    case "Idle":
      return "Not armed";
    case "Sighting":
      return "Click into Path of Exile";
    case "Ready":
      return `Tap ${status.triggerName} to roll`;
    case "Rolling":
      return "Rolling";
    case "Resyncing":
      return `Tap ${status.triggerName} to re-read`;
    case "Latched":
      return "Hit — stop rolling";
    case "Halted":
      return "Stopped — something is wrong";
  }
};

/** One clause of context under the headline. Still short enough to read in a glance. */
export const subhead = (status: CycleStatus): string => {
  switch (status.state) {
    case "Idle":
      return "Choose a target mod, then arm.";
    case "Sighting":
      return "The first press reads the jewel and spends no orb.";
    case "Ready":
      return status.lastTier === null
        ? "The target mod is not on this jewel."
        : `Last roll was tier ${status.lastTier}.`;
    case "Rolling":
      return "";
    case "Resyncing":
      return `Lost track of the jewel — ${status.consecutiveUnknown} of ${status.unknownLimit}. Re-reading spends no orb.`;
    case "Latched":
      return "The next press will not roll. Acknowledge below when the orb is off your cursor.";
    case "Halted":
      return "Re-arm once you have fixed it.";
  }
};

/**
 * The checklist core currently buries inside a paragraph.
 *
 * `Sighting` hides an ordered three-step procedure in one sentence, and the order matters — a
 * Shift-click with an orb on the cursor applies that orb, which is why "Shift NOT held" comes first
 * and is not a footnote. `Halted` on an Unknown run hides three yes/no checks in the middle of an
 * explanation. Both are lists pretending to be prose.
 */
export const steps = (status: CycleStatus): string[] => {
  switch (status.state) {
    case "Sighting":
      return [
        "Click into Path of Exile — with Shift NOT held, or you will apply the orb you are holding",
        "Hold Shift and hover the jewel",
        `Tap ${status.triggerName} once — this press only reads, and spends no orb`,
      ];
    case "Halted":
      // Only the Unknown-run Halt has a checklist. A wrong-item Halt is a statement, not a
      // procedure, so it gets none and falls through to core's own sentence.
      return status.haltReason?.includes("Unknown Verdict")
        ? [
            "Is the jewel still hovered?",
            "Is an Orb of Alteration still on the cursor?",
            "Is Shift still held?",
          ]
        : [];
    default:
      return [];
  }
};

/** How loud a state should look. Same three buckets the shipped panel uses. */
export const tone = (status: CycleStatus): "good" | "bad" | "hit" | "flat" => {
  switch (status.state) {
    case "Latched":
      return "hit";
    case "Halted":
    case "Resyncing":
      return "bad";
    case "Idle":
      return "flat";
    default:
      return "good";
  }
};
