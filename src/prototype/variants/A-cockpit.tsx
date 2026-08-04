/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * # A · Cockpit — "nothing was wrong except the ranking"
 *
 * The one variant that changes **no copy at all**. Core's `status.message` is rendered verbatim and in
 * full, exactly as the shipped window does; the trigger-key field, the hook counters, the Sticky Keys
 * badges, the timings and the anchor are all still here and still reachable. The only thing that
 * changes is *rank*: one thing is the biggest thing on screen, one thing is the primary control, and
 * every diagnostic is folded into a single collapsed strip at the bottom.
 *
 * It exists to answer a question that would otherwise stay open — **how much of "walls of text and
 * debug stuff" is layout, and how much is the words themselves?** If A is enough, #9 is a CSS ticket
 * and `crates/core` never has to change. If it is not, that is the evidence that some of the answer
 * has to be a change to core's strings, which is what B, C and D assume.
 *
 * Threshold control: the **tier dropdown**, unchanged from the shipped panel — so A holds that
 * variable still while B, C and D each move it.
 */
import { bands, cumulative, label, odds, percent } from "../data";
import type { VariantProps } from "./types";

export const name = "Cockpit — same information, re-ranked";

export default function VariantA({
  status,
  pool,
  group,
  ilvl,
  setIlvl,
  lastRead,
  setTarget,
  setArmed,
  acknowledge,
}: VariantProps) {
  const armed = status.state !== "Idle";
  const chance = group ? odds(group, status.tierThreshold, ilvl) : null;
  const prefixes = pool.groups.filter((g) => g.generation === "prefix");
  const suffixes = pool.groups.filter((g) => g.generation === "suffix");
  const accessibility = status.accessibility;

  return (
    <div className="pv-a">
      {/* The state, and core's own sentence. Same words as today, given the top of the page and a
          size that survives being glanced at from a second monitor. */}
      <div className={`pv-a-band pv-tone-${status.state === "Latched" ? "hit" : status.state === "Halted" || status.state === "Resyncing" ? "bad" : armed ? "good" : "flat"}`}>
        <div className="pv-a-state">{status.state}</div>
        <p className="pv-a-message">{status.message}</p>
      </div>

      {/* The primary control, immediately under the state that demands it — never further down the
          page than the sentence telling you to use it. */}
      <div className="pv-a-act">
        {status.state === "Latched" ? (
          <button className="pv-btn pv-btn-hit" onClick={acknowledge}>
            Acknowledge the Hit
          </button>
        ) : status.state === "Halted" ? (
          <button className="pv-btn pv-btn-primary" onClick={() => setArmed(true)}>
            Re-arm
          </button>
        ) : (
          <button
            className={armed ? "pv-btn" : "pv-btn pv-btn-primary"}
            onClick={() => setArmed(!armed)}
          >
            {armed ? "Stop" : "Arm"}
          </button>
        )}
        <div className="pv-a-numbers">
          <span>
            <strong>{status.rolls}</strong> alterations
          </span>
          {chance && !chance.impossible && (
            <span>
              <strong>{percent(cumulative(chance.perClick, status.rolls), 1)}</strong> chance by now
            </span>
          )}
          {chance && !chance.impossible && (
            <span>
              median <strong>{chance.median}</strong>
            </span>
          )}
        </div>
      </div>

      {/* The target, as a sentence rather than a form, once it is locked in. */}
      <div className="pv-a-target">
        {group ? (
          <p>
            Looking for <strong>{label(group)}</strong> at <strong>Tier {status.tierThreshold}</strong>{" "}
            or better — {bands(group, status.tierThreshold)}.{" "}
            {chance && !chance.impossible ? (
              <>
                About <strong>1 in {chance.oneIn}</strong> rolls.
              </>
            ) : (
              <span className="pv-warn">
                No tier that good can spawn on an item level {ilvl} jewel.
              </span>
            )}
          </p>
        ) : (
          <p className="pv-muted">No target mod chosen.</p>
        )}

        <div className="pv-a-picker">
          <label>
            <span>Mod group</span>
            <select
              disabled={armed}
              value={status.targetGroup}
              onChange={(event) => setTarget(event.target.value, status.tierThreshold)}
            >
              <optgroup label={`Prefixes (${prefixes.length})`}>
                {prefixes.map((g) => (
                  <option key={g.id} value={g.id}>
                    {label(g)}
                  </option>
                ))}
              </optgroup>
              <optgroup label={`Suffixes (${suffixes.length})`}>
                {suffixes.map((g) => (
                  <option key={g.id} value={g.id}>
                    {label(g)}
                  </option>
                ))}
              </optgroup>
            </select>
          </label>
          <label>
            <span>Tier threshold</span>
            <select
              disabled={armed || !group}
              value={status.tierThreshold}
              onChange={(event) => setTarget(status.targetGroup, Number(event.target.value))}
            >
              {group?.tiers.map((tier) => (
                <option key={tier.tier} value={tier.tier}>
                  T{tier.tier} · {bands(group, tier.tier)} · ilvl {tier.requiredIlvl}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Jewel item level</span>
            <input
              type="number"
              min={1}
              max={100}
              value={ilvl}
              disabled={armed}
              onChange={(event) => setIlvl(Number(event.target.value))}
            />
          </label>
        </div>
      </div>

      {lastRead && (
        <details className="pv-a-fold">
          <summary>Last read</summary>
          <pre>{lastRead}</pre>
        </details>
      )}

      {/* Everything that is only ever interesting when something is wrong. One fold, at the bottom,
          shut by default — but never removed, because on a machine with no dev tools this is the
          entire diagnostic surface. A red dot on the summary is the compromise: out of the way, but it
          tells you to open it. */}
      <details className="pv-a-fold" open={status.state === "Halted"}>
        <summary>
          Diagnostics
          {(!status.running || status.keysSeen === 0 || accessibility?.stickyKeysOn) && (
            <span className="pv-dot" />
          )}
        </summary>
        <div className="pv-a-diag">
          <span className={status.running ? "pv-ok" : "pv-bad"}>
            WH_KEYBOARD_LL {status.running ? "installed" : "NOT installed"}
          </span>
          <span className={status.keysSeen > 0 ? "pv-ok" : "pv-bad"}>
            {status.keysSeen} keystrokes seen
          </span>
          <span className={status.shiftDown ? "pv-ok" : ""}>
            Shift {status.shiftDown ? "held" : "up"}
          </span>
          <span className={accessibility?.stickyKeysOn ? "pv-bad" : ""}>
            Sticky Keys {accessibility?.stickyKeysOn ? "ON" : "off"}
          </span>
          <span>{status.anchor ? `anchor ${status.anchor[0]},${status.anchor[1]}` : "no anchor"}</span>
          <span>{status.presses} presses</span>
          <span>{status.pressesDropped} dropped mid-cycle</span>
          <span>
            cycle {status.cycleMs ?? "–"}ms · copy {status.copyMs ?? "–"}ms
          </span>
          <span className="pv-mono">{status.foreground}</span>
          <span>
            trigger {status.triggerName} ({status.triggerVk})
          </span>
        </div>
        {status.keysSeen === 0 && status.running && (
          <p className="pv-warn">
            The hook is installed but has seen no keystroke. Press any key — if this stays at 0 the
            hook is deaf.
          </p>
        )}
        {accessibility?.stickyKeysOn && (
          <p className="pv-warn">
            Sticky Keys is on. It changes what holding Shift means, with no visible sign, so Apply Mode
            will drop out mid-craft.
          </p>
        )}
      </details>
    </div>
  );
}
