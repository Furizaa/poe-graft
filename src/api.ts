/**
 * The Rust side of the seam, as seen from TypeScript.
 *
 * ADR 0001 keeps this contract tiny on purpose: the frontend holds no domain logic. Right now
 * the whole surface is provenance, one platform read, and the log — the roll cycle's payloads
 * (`{ targetModGroup, maxTier, triggerKey, itemPosition }` down, `{ rollCount, lastRoll, state }`
 * up) arrive when the cycle itself does.
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
 * Every updater event goes through here. On a machine with no dev environment the log is the
 * only thing that survives a relaunch, and the updater force-exits the app mid-install — so
 * anything not written down is lost exactly when it matters.
 */
export const appendLog = (line: string) => invoke<void>("log_append", { line });

// -----------------------------------------------------------------------------------------------
// The throwaway on-device spike — https://github.com/Furizaa/poe-graft/issues/17
//
// This whole block goes away with the spike. It is not the roll cycle's contract: that is
// `{ targetModGroup, maxTier, triggerKey, itemPosition }` down and `{ rollCount, lastRoll, state }`
// up, and it arrives when #7 designs the cycle.
// -----------------------------------------------------------------------------------------------

/** What one completed roll produced. */
export type SpikeRoll = {
  roll: number;
  /** `Ctrl+C` → clipboard changed. The number to compare against AutoHotkey's 15–32 ms. */
  copyMs: number;
  /** Click → text in hand, i.e. `copyMs` plus the settle delay. */
  cycleMs: number;
  timedOut: boolean;
  /** The clipboard changed but still held our sentinel — something else wrote to it. */
  stale: boolean;
  /** Byte-identical to the previous roll, which usually means the settle delay is too short. */
  identicalToPrevious: boolean;
  shiftDown: boolean;
  chars: number;
  summary: string;
};

/** Accessibility settings that silently change how a held modifier behaves. */
export type SpikeAccessibility = {
  stickyKeysOn: boolean;
  stickyKeysAvailable: boolean;
  filterKeysOn: boolean;
  toggleKeysOn: boolean;
};

export type SpikeStatus = {
  /** False on macOS, where every other field is meaningless. */
  supported: boolean;
  hookInstalled: boolean;
  armed: boolean;
  learning: boolean;
  suppress: boolean;
  releaseShift: boolean;
  guardForeground: boolean;
  triggerVk: number;
  triggerName: string;
  lastKeyVk: number;
  lastKeyName: string;
  /**
   * Physical key-downs the hook callback has observed since install. A count only — never which
   * keys, unless learning is on.
   *
   * This is the decisive diagnostic: `SetWindowsHookExW` returning a handle proves Windows
   * accepted the hook, not that it delivers anything. Installed with `keysSeen` stuck at 0 means
   * the hook is deaf, which is a different fault from the panel failing to render what it heard.
   */
  keysSeen: number;
  position: [number, number] | null;
  rolls: number;
  maxRolls: number;
  copyDelayMs: number;
  readTimeoutMs: number;
  tolerancePx: number;
  /** Physical presses seen while armed. A growing gap against `rolls` is fail-closed sequencing. */
  presses: number;
  shiftDown: boolean;
  foreground: string;
  lastRoll: SpikeRoll | null;
  accessibility: SpikeAccessibility | null;
};

/** Sent whole whenever any single field changes, so there is one way in rather than nine. */
export type SpikeConfig = {
  triggerVk: number;
  learning: boolean;
  suppress: boolean;
  releaseShift: boolean;
  guardForeground: boolean;
  copyDelayMs: number;
  readTimeoutMs: number;
  tolerancePx: number;
  maxRolls: number;
};

export const getSpikeStatus = () => invoke<SpikeStatus>("spike_status");

export const setSpikeHook = (on: boolean) => invoke<void>("spike_hook", { on });

export const setSpikeArmed = (on: boolean) => invoke<void>("spike_arm", { on });

export const configureSpike = (config: SpikeConfig) =>
  invoke<void>("spike_configure", { config });

export const forgetSpikePosition = () => invoke<void>("spike_forget_position");

/** Put one of the human's own observations into the machine's log, in order, timestamped. */
export const noteSpike = (line: string) => invoke<void>("spike_note", { line });
