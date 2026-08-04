/**
 * The Rust side of the seam, as seen from TypeScript.
 *
 * ADR 0001 keeps this contract tiny on purpose: **the frontend holds no domain logic.** Every
 * Verdict, every Refusal, every Halt and every line of copy explaining them is Rust's. What crosses
 * here is the Target Mod and the Trigger Key going down, and a status readout coming up.
 */
import { invoke } from "@tauri-apps/api/core";

/** Provenance for the running binary, resolved at compile time. */
export type BuildInfo = {
  /** The version the updater compares. */
  version: string;
  /** Short commit sha, or `"local"` for a development-machine build. */
  commit: string;
  /** Actions run number, absent for local builds. */
  runNumber: string | null;
  /** Link to the Actions run that produced this binary. */
  runUrl: string | null;
  /** Which `Platform` implementation is wired in. */
  platform: string;
};

/** A liveness readout that could only have come from a real Win32 call. */
export type PlatformInfo = {
  screenWidth: number;
  screenHeight: number;
  cursorX: number;
  cursorY: number;
};

export const getBuildInfo = () => invoke<BuildInfo>("build_info");

export const getPlatformInfo = () => invoke<PlatformInfo>("platform_info");

export const getLogPath = () => invoke<string>("log_path");

export const getLogTail = () => invoke<string[]>("log_tail");

/**
 * Append a line to the same file Rust writes to.
 *
 * Every updater event goes through here. On a machine with no dev environment the log is the only
 * thing that survives a relaunch, and the updater force-exits the app mid-install — so anything not
 * written down is lost exactly when it matters.
 */
export const appendLog = (line: string) => invoke<void>("log_append", { line });

// -----------------------------------------------------------------------------------------------
// The mod pool — read once at startup from data/ghastly-eye-jewel.json
// -----------------------------------------------------------------------------------------------

/** One tier of one Mod Group, so the human can see what a Tier Threshold actually buys. */
export type ModTier = {
  /** 1 is the best. */
  tier: number;
  /** What the game calls *this* tier of the group — `Flaring` at T1, `Annealed` at T4. */
  affixName: string;
  /** The lowest item level this tier can spawn on. The map's target needs 83. */
  requiredIlvl: number;
  /** `[min, max]` per rolled value, in the order they are printed. */
  bands: [number, number][];
};

/** A family of modifiers spanning every tier of it. */
export type ModGroup = {
  /** The stable identifier a Target Mod is stored as. */
  id: string;
  generation: "prefix" | "suffix";
  /**
   * The rendered lines, values replaced by `#`. **This is the label.**
   *
   * A group is never named by an affix name, because that name is per tier — the same group is
   * `Annealed` at T4 and `Flaring` at T1, which is the trap issue #4 walked into.
   */
  lines: string[];
  /** Every tier, best first. */
  tiers: ModTier[];
};

export type ModPool = {
  baseName: string;
  itemClass: string;
  groups: ModGroup[];
};

/** The tier data. Rejects if it failed to load, and then nothing can be crafted. */
export const getModPool = () => invoke<ModPool>("mod_pool");

/** Which file the tier data came from — unanswerable any other way on the gaming PC. */
export const getModPoolSource = () => invoke<string>("mod_pool_source");

// -----------------------------------------------------------------------------------------------
// The roll cycle
// -----------------------------------------------------------------------------------------------

/** The accessibility settings that silently change how a held modifier behaves. */
export type Accessibility = {
  stickyKeysOn: boolean;
  stickyKeysAvailable: boolean;
  filterKeysOn: boolean;
  toggleKeysOn: boolean;
};

/** One of ADR 0002's seven states. */
export type CycleState =
  | "Idle"
  | "Sighting"
  | "Ready"
  | "Rolling"
  | "Resyncing"
  | "Latched"
  | "Halted";

export type CycleStatus = {
  /** False on macOS, where the hook, the injection and the clipboard are all absent. */
  supported: boolean;
  /** Is the hook installed and the worker alive? */
  running: boolean;
  state: CycleState;
  /**
   * The most recent thing worth saying — core's own words.
   *
   * Rendered verbatim, never paraphrased: it is the same string the log file got, and on a machine
   * with no dev tools two different wordings of the same event is a bug you cannot debug.
   */
  message: string;
  targetGroup: string;
  tierThreshold: number;
  /** Alterations spent this Craft Session. */
  rolls: number;
  anchor: [number, number] | null;
  /** Presses dropped mid-cycle. A growing number is fail-closed sequencing, not a bug. */
  pressesDropped: number;
  consecutiveUnknown: number;
  unknownLimit: number;
  lastVerdict: "Hit" | "Miss" | "Unknown" | null;
  lastTier: number | null;
  /** Why the session Halted. Shown verbatim — after a Halt the human needs to read *why*. */
  haltReason: string | null;
  /** Is the Trigger Key being swallowed? On exactly while a Craft Session is armed. */
  suppress: boolean;
  triggerVk: number;
  triggerName: string;
  /**
   * Physical key-downs the hook callback has seen since it was installed.
   *
   * The decisive diagnostic: `SetWindowsHookExW` returning a handle proves Windows accepted the
   * hook, not that it delivers anything. Running with `keysSeen` stuck at 0 means the hook is deaf,
   * which is a different fault from the window failing to render what it heard.
   */
  keysSeen: number;
  /** Physical Trigger Presses seen while armed. */
  presses: number;
  shiftDown: boolean;
  foreground: string;
  /** Monotonic counters. One sound per increment. */
  hits: number;
  halts: number;
  blips: number;
  /** `Ctrl+C` → the clipboard changed. Our own code does this in 1–8 ms. */
  copyMs: number | null;
  /** The whole last cycle, click to text in hand. */
  cycleMs: number | null;
  accessibility: Accessibility | null;
};

export const getCycleStatus = () => invoke<CycleStatus>("cycle_status");

/** Arm or disarm a Craft Session. Arming is a mouse click, which is why #18 matters. */
export const setArmed = (on: boolean) => invoke<void>("cycle_arm", { on });

/**
 * Acknowledge a Latched Hit.
 *
 * The only thing that releases a Latch, and deliberately a mouse click in this window: a key that
 * can clear a Hit is a key your reflexes can clear a Hit with.
 */
export const acknowledge = () => invoke<void>("cycle_acknowledge");

/** Choose the Target Mod. Rejects while a Craft Session is armed. */
export const setTarget = (groupId: string, tierThreshold: number) =>
  invoke<void>("cycle_set_target", { groupId, tierThreshold });

/** Choose the Trigger Key by virtual-key code. */
export const setTrigger = (vk: number) => invoke<void>("cycle_set_trigger", { vk });

/** Put one of the human's own observations into the machine's log, in order, timestamped. */
export const note = (line: string) => invoke<void>("cycle_note", { line });
