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
