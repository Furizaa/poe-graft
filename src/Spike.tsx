/**
 * The control panel for the throwaway on-device spike,
 * [#17](https://github.com/Furizaa/poe-graft/issues/17).
 *
 * This file goes away with the spike. It is not a sketch of the real UI — that is
 * [#9](https://github.com/Furizaa/poe-graft/issues/9) — it is an instrument panel, laid out in the
 * order the session runs: check the environment, choose a key, set the bounds, then arm.
 *
 * Two things it deliberately does *not* do. It never decides anything: every verdict is Rust's,
 * and the panel only shows what Rust reports. And it never paraphrases an error — on a machine
 * with no dev tools, the raw string is the only diagnostic there is.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import {
  configureSpike,
  forgetSpikePosition,
  getSpikeStatus,
  noteSpike,
  setSpikeArmed,
  setSpikeHook,
  type SpikeConfig,
  type SpikeStatus,
} from "./api";

const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

/** How often to re-poll. Fast enough that a roll appears as you press, cheap enough to ignore. */
const POLL_MS = 350;

const configOf = (status: SpikeStatus): SpikeConfig => ({
  triggerVk: status.triggerVk,
  learning: status.learning,
  suppress: status.suppress,
  releaseShift: status.releaseShift,
  guardForeground: status.guardForeground,
  copyDelayMs: status.copyDelayMs,
  readTimeoutMs: status.readTimeoutMs,
  tolerancePx: status.tolerancePx,
  maxRolls: status.maxRolls,
  badLimit: status.badLimit,
});

function Flag({ label, on, warn }: { label: string; on: boolean; warn?: boolean }) {
  const tone = on ? (warn ? "badge bad" : "badge good") : "badge";
  return (
    <span className={tone}>
      {label} {on ? "ON" : "off"}
    </span>
  );
}

function NumberField({
  label,
  hint,
  value,
  onChange,
  disabled,
}: {
  label: string;
  hint: string;
  value: number;
  onChange: (value: number) => void;
  disabled?: boolean;
}) {
  return (
    <label className="field">
      <span className="muted small">{label}</span>
      <input
        type="number"
        value={value}
        disabled={disabled}
        onChange={(event) => {
          const next = Number(event.target.value);
          if (Number.isFinite(next)) onChange(next);
        }}
      />
      <span className="muted small">{hint}</span>
    </label>
  );
}

function Toggle({
  label,
  hint,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  hint: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <label className="toggle">
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span>
        {label} <span className="muted small">— {hint}</span>
      </span>
    </label>
  );
}

function LastRoll({ status }: { status: SpikeStatus }) {
  const roll = status.lastRoll;
  if (!roll) {
    return (
      <p className="muted small">
        No roll yet. Hold Shift with the orbs in your inventory, hover the jewel, and press the
        trigger — the first press captures the position, the second rolls.
      </p>
    );
  }

  const verdict = roll.timedOut
    ? "TIMED OUT"
    : roll.stale
      ? "STALE"
      : roll.identicalToPrevious
        ? "IDENTICAL to the previous roll"
        : "OK";

  return (
    <>
      <p className="mono">
        roll {roll.roll} · <strong>{verdict}</strong> · copy {roll.copyMs}ms · cycle{" "}
        {roll.cycleMs}ms · {roll.chars} chars · shift {roll.shiftDown ? "down" : "UP"}
      </p>
      {roll.summary && <p className="mono small">{roll.summary}</p>}
      {roll.identicalToPrevious && !roll.timedOut && (
        <p className="error small">
          Two identical reads in a row. Either the roll genuinely repeated, or the settle delay is
          too short and the copy is reading the item as it was before the click. Raise the delay
          and see whether it stops.
        </p>
      )}
    </>
  );
}

export default function Spike({ refreshLog }: { refreshLog: () => Promise<void> }) {
  const [status, setStatus] = useState<SpikeStatus | null>(null);
  const [config, setConfig] = useState<SpikeConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState("");
  /** Rust owns the defaults, so the panel adopts them once and then stops overwriting itself. */
  const adopted = useRef(false);
  /** Refresh the log only when a roll actually lands, rather than on every poll. */
  const lastSeenRoll = useRef(-1);

  useEffect(() => {
    let live = true;
    const poll = async () => {
      try {
        const next = await getSpikeStatus();
        if (!live) return;
        setStatus(next);
        if (!adopted.current) {
          adopted.current = true;
          setConfig(configOf(next));
        }
        if (next.rolls !== lastSeenRoll.current) {
          lastSeenRoll.current = next.rolls;
          await refreshLog();
        }
      } catch (caught) {
        if (live) setError(errorText(caught));
      }
    };
    void poll();
    const timer = setInterval(() => void poll(), POLL_MS);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, [refreshLog]);

  /** Push the whole config down, then let the next poll confirm what Rust actually accepted. */
  const push = useCallback(
    async (next: SpikeConfig) => {
      setConfig(next);
      try {
        await configureSpike(next);
        setError(null);
      } catch (caught) {
        setError(errorText(caught));
      }
    },
    [],
  );

  const act = useCallback(
    async (action: () => Promise<void>) => {
      try {
        await action();
        setError(null);
      } catch (caught) {
        setError(errorText(caught));
      }
      await refreshLog();
    },
    [refreshLog],
  );

  if (!status || !config) {
    return (
      <section>
        <h2>Spike — issue #17</h2>
        <p className="muted">Loading…</p>
      </section>
    );
  }

  if (!status.supported) {
    return (
      <section>
        <h2>Spike — issue #17</h2>
        <p className="muted small">
          The spike only runs on Windows. On the development machine there is no hook, no game and
          no clipboard worth poisoning — this panel is here so the layout can be checked, and it
          will report real numbers on the gaming PC.
        </p>
      </section>
    );
  }

  const accessibility = status.accessibility;

  return (
    <section>
      <h2>Spike — issue #17</h2>

      {/* Step 1. Sticky Keys is checked first because it would silently corrupt the other two
          answers, and because a machine that has it on invalidates the whole session. */}
      <h3>1 · Environment</h3>
      <div className="row">
        {accessibility ? (
          <>
            <Flag label="Sticky Keys" on={accessibility.stickyKeysOn} warn />
            <Flag label="Filter Keys" on={accessibility.filterKeysOn} warn />
            <Flag label="Toggle Keys" on={accessibility.toggleKeysOn} />
            <span className="badge">
              five-taps shortcut {accessibility.stickyKeysAvailable ? "ENABLED" : "off"}
            </span>
          </>
        ) : (
          <span className="error">Could not read the accessibility settings.</span>
        )}
        <Flag label="Shift" on={status.shiftDown} />
      </div>
      {accessibility?.stickyKeysOn && (
        <p className="error">
          Sticky Keys is on. It changes what holding Shift means, with no error and no visible
          sign, so every answer this session produces would be suspect. Turn it off in Settings →
          Accessibility → Keyboard before going any further.
        </p>
      )}
      {accessibility?.stickyKeysAvailable && !accessibility.stickyKeysOn && (
        <p className="muted small">
          Sticky Keys is off but its five-taps-on-Shift shortcut is enabled — which a
          Shift-heavy crafting session can trip by accident. Worth disabling the shortcut too.
        </p>
      )}
      <p className="muted small mono">foreground {status.foreground}</p>

      {/* Step 2. The hook has to exist before a key can be learned, because learning *is* the
          hook reporting what it saw. */}
      <h3>2 · Keyboard hook</h3>
      <div className="row">
        <button
          onClick={() => void act(() => setSpikeHook(!status.hookInstalled))}
          className={status.hookInstalled ? "" : "primary"}
        >
          {status.hookInstalled ? "Remove hook" : "Install hook"}
        </button>
        <span className="badge">
          WH_KEYBOARD_LL {status.hookInstalled ? "installed" : "not installed"}
        </span>
        {/* The decisive diagnostic. Installing only proves Windows accepted the hook; this proves
            it is delivering. Press any key and watch it climb. */}
        <span
          className={
            status.hookInstalled
              ? status.keysSeen > 0
                ? "badge good"
                : "badge bad"
              : "badge"
          }
        >
          {status.keysSeen} keystrokes seen
        </span>
      </div>
      {status.hookInstalled && status.keysSeen === 0 && (
        <p className="error">
          The hook is installed but has not seen a single keystroke. Press any key now — this
          should climb immediately. If it stays at 0, the hook is deaf rather than the panel being
          wrong, and no amount of pressing keys will help: report it and stop here.
        </p>
      )}
      {status.hookInstalled && status.keysSeen > 0 && (
        <p className="muted small">
          The hook is delivering. This counts key-downs only, and never records which keys unless
          you turn learning on below.
        </p>
      )}

      {/* Step 3. The trigger is learned rather than guessed: this keyboard has no F-row, so the
          F13–F24 keys PoE ignores are not available and the right key is whatever is spare. */}
      <h3>3 · Trigger key</h3>
      <div className="row">
        <button
          disabled={!status.hookInstalled}
          onClick={() => void push({ ...config, learning: !config.learning })}
        >
          {status.learning ? "Stop learning" : "Learn a key"}
        </button>
        <button
          disabled={!status.lastKeyVk || status.lastKeyVk === status.triggerVk}
          onClick={() =>
            void push({ ...config, triggerVk: status.lastKeyVk, learning: false })
          }
        >
          Use this key
        </button>
        <span className={status.lastKeyVk ? "badge good" : "badge"}>
          last key {status.lastKeyName}
        </span>
        <span className={status.triggerVk ? "badge good" : "badge"}>
          trigger {status.triggerName}
        </span>
      </div>
      {status.learning && (
        <p className="muted small">
          Press the key you want to use — <strong>last key</strong> above should change. Key codes
          are only recorded while this is on.
        </p>
      )}

      {/* A way through that does not depend on learning working. Learning is the nicer path, but
          it is not the only one, and the whole session would otherwise be gated on it. */}
      <div className="row">
        <NumberField
          label="Or set the code directly"
          hint="decimal virtual-key code"
          value={config.triggerVk}
          onChange={(triggerVk) => void push({ ...config, triggerVk })}
        />
        <span className="muted small">
          Scroll Lock <span className="mono">145</span> · Pause <span className="mono">19</span> ·
          Insert <span className="mono">45</span> · Home <span className="mono">36</span> · End{" "}
          <span className="mono">35</span> · Page Up <span className="mono">33</span> · Page Down{" "}
          <span className="mono">34</span> · Numpad 0 <span className="mono">96</span> · <span
            className="mono"
          >
            `
          </span>{" "}
          <span className="mono">192</span>
        </span>
      </div>

      <h3>4 · Bounds</h3>
      <div className="row">
        <NumberField
          label="Settle delay"
          hint="ms, click → Ctrl+C"
          value={config.copyDelayMs}
          onChange={(copyDelayMs) => void push({ ...config, copyDelayMs })}
        />
        <NumberField
          label="Read timeout"
          hint="ms"
          value={config.readTimeoutMs}
          onChange={(readTimeoutMs) => void push({ ...config, readTimeoutMs })}
        />
        <NumberField
          label="Drift tolerance"
          hint="px"
          value={config.tolerancePx}
          onChange={(tolerancePx) => void push({ ...config, tolerancePx })}
        />
        <NumberField
          label="Roll cap"
          hint="then disarms"
          value={config.maxRolls}
          onChange={(maxRolls) => void push({ ...config, maxRolls })}
        />
        <NumberField
          label="Stop after"
          hint="bad reads in a row"
          value={config.badLimit}
          onChange={(badLimit) => void push({ ...config, badLimit })}
        />
      </div>
      <div className="options">
        <Toggle
          label="Suppress the trigger key"
          hint="does swallowing it reach PoE, which reads the mouse via Raw Input?"
          checked={config.suppress}
          onChange={(suppress) => void push({ ...config, suppress })}
        />
        <Toggle
          label="Require Path of Exile in the foreground"
          hint="off means a click can land in another window — leave it on"
          checked={config.guardForeground}
          onChange={(guardForeground) => void push({ ...config, guardForeground })}
        />
        <Toggle
          label="Copy mode B — release Shift around Ctrl+C"
          hint="the fallback, and it ends apply mode every roll. Only try it if mode A fails"
          checked={config.releaseShift}
          onChange={(releaseShift) => void push({ ...config, releaseShift })}
        />
      </div>

      <h3>5 · Run</h3>
      <div className="row">
        <button
          disabled={!status.hookInstalled || !status.triggerVk}
          className={status.armed ? "" : "primary"}
          onClick={() => void act(() => setSpikeArmed(!status.armed))}
        >
          {status.armed ? "Disarm" : "Arm"}
        </button>
        <button disabled={!status.position} onClick={() => void act(forgetSpikePosition)}>
          Forget position
        </button>
        <span className={status.armed ? "badge good" : "badge"}>
          {status.armed ? "ARMED" : "disarmed"}
        </span>
        <span className="badge">
          {status.position ? `item at ${status.position[0]},${status.position[1]}` : "no position"}
        </span>
        <span className="badge">
          {status.rolls} / {status.maxRolls} rolls
        </span>
        <span className="badge">{status.presses} presses</span>
      </div>
      {status.presses > status.rolls + 1 && (
        <p className="muted small">
          {status.presses - status.rolls - 1} press(es) did not become a roll. Every one is a line
          in the log saying why it was refused — that is fail-closed sequencing, not a bug.
        </p>
      )}

      <h3>6 · Last roll</h3>
      <LastRoll status={status} />

      {/* The human's own observations belong in the same file, in order, timestamped — otherwise
          "apply mode dropped out around roll 30" has to be reconstructed from memory later. */}
      <h3>7 · Note what you saw</h3>
      <div className="row">
        <input
          className="grow"
          value={note}
          placeholder="e.g. apply mode dropped out at roll 31"
          onChange={(event) => setNote(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && note.trim()) {
              const line = note.trim();
              setNote("");
              void act(() => noteSpike(line));
            }
          }}
        />
        <button
          disabled={!note.trim()}
          onClick={() => {
            const line = note.trim();
            setNote("");
            void act(() => noteSpike(line));
          }}
        >
          Log it
        </button>
      </div>

      {error && <p className="error">{error}</p>}
    </section>
  );
}
