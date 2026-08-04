/** PROTOTYPE — throwaway. See `src/prototype/README.md`. */
import type { CycleStatus, ModGroup, ModPool } from "../../api";

/**
 * What every variant gets.
 *
 * Read-only except for the four actions, which are stubs in the prototype — `UI.md` is explicit that
 * a prototype should not be wired to real mutations, and on macOS `set_target` and `cycle_arm` reject
 * anyway.
 */
export type VariantProps = {
  status: CycleStatus;
  pool: ModPool;
  /** The currently targeted group, resolved from the pool. Null before one is chosen. */
  group: ModGroup | null;
  /**
   * The item level the odds are computed against.
   *
   * A finding rather than a given: **the odds move with item level and the shipped window has no item
   * level in it.** Mid-session this is knowable for free — the parser already reads `Item Level:` off
   * the Item Text — but before arming the UI has to assume one. Each variant takes a different
   * position on whether to show it at all.
   */
  ilvl: number;
  setIlvl: (ilvl: number) => void;
  /** The last Item Text read, when there is one. Real capture content, never invented. */
  lastRead: string | null;
  /** Core's own log lines, newest last. */
  logLines: string[];
  setTarget: (groupId: string, tierThreshold: number) => void;
  setArmed: (on: boolean) => void;
  acknowledge: () => void;
};
