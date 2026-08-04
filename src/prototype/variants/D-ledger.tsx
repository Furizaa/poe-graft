/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * # D · Ledger — "text is not the problem; paragraphs are"
 *
 * The one variant that argues *against* the obvious reading of the brief. The owner said walls of text,
 * and the reflex is to show less text — but core already emits a stream of short, ordered, timestamped
 * lines, and the shipped window puts that stream in a `<pre>` at the very **bottom** of the page,
 * below the updater and the platform seam, where nobody looks mid-craft. So the window shows its
 * paragraphs prominently and its scannable lines nowhere useful.
 *
 * D inverts that: a thin sticky status strip, and underneath it the log as the primary surface,
 * newest first, one line per event, keyed by kind. Nothing is summarised and nothing is paraphrased —
 * these are core's own words, which is also what makes this the only variant that provably cannot
 * disagree with the log file.
 *
 * The premise is [#11](https://github.com/Furizaa/poe-graft/issues/11)'s: on a machine with no dev
 * tools the log *is* the UI, and it is the only artifact that survives a relaunch. If D wins, #9 and
 * #11 collapse into one ticket.
 *
 * Threshold control: **radio rows, one per tier, each carrying its own odds** — so the choice is made
 * against the number it costs rather than by picking a tier and then reading a consequence somewhere
 * else. This is the only variant where all six tiers' odds are visible at once.
 */
import { bands, cumulative, label, odds, percent } from "../data";
import type { VariantProps } from "./types";

export const name = "Ledger — the log is the UI, with per-tier odds";

/** What kind of line this is, for colour only. Never changes what it says. */
const kindOf = (line: string) => {
  if (line.startsWith("HIT")) return "hit";
  if (line.startsWith("HALTED")) return "halt";
  if (line.startsWith("Unknown")) return "unknown";
  if (line.startsWith("Refused")) return "refused";
  if (line.startsWith("Diagnostic")) return "diag";
  if (line.startsWith("Miss")) return "miss";
  if (line.startsWith("Roll ")) return "roll";
  if (line.startsWith("────")) return "rule";
  return "plain";
};

export default function VariantD({
  status,
  pool,
  group,
  ilvl,
  setIlvl,
  logLines,
  setTarget,
  setArmed,
  acknowledge,
}: VariantProps) {
  const armed = status.state !== "Idle";
  const chance = group ? odds(group, status.tierThreshold, ilvl) : null;

  return (
    <div className="pv-d">
      {/* One line, always visible, never scrolls away. Everything on it is a number or a verb. */}
      <div
        className={`pv-d-strip pv-tone-${status.state === "Latched" ? "hit" : status.state === "Halted" || status.state === "Resyncing" ? "bad" : armed ? "good" : "flat"}`}
      >
        <span className="pv-d-state">{status.state}</span>
        <span className="pv-d-num">
          <strong>{status.rolls}</strong> rolls
        </span>
        {chance && !chance.impossible && (
          <span className="pv-d-num">
            <strong>{percent(cumulative(chance.perClick, status.rolls), 1)}</strong> by now
          </span>
        )}
        {status.cycleMs !== null && (
          <span className="pv-d-num pv-mono">
            {status.cycleMs}ms · copy {status.copyMs}ms
          </span>
        )}
        <span className="pv-d-spacer" />
        {status.state === "Latched" ? (
          <button className="pv-btn pv-btn-hit" onClick={acknowledge}>
            Acknowledge
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
      </div>

      {/* Setup, only when idle — the ledger takes the whole window once a session is running. */}
      {!armed && (
        <div className="pv-d-setup">
          <label className="pv-d-groupsel">
            <span>Target mod group</span>
            <select
              value={status.targetGroup}
              onChange={(event) => setTarget(event.target.value, 1)}
            >
              <optgroup label="Prefixes">
                {pool.groups
                  .filter((g) => g.generation === "prefix")
                  .map((g) => (
                    <option key={g.id} value={g.id}>
                      {label(g)}
                    </option>
                  ))}
              </optgroup>
              <optgroup label="Suffixes">
                {pool.groups
                  .filter((g) => g.generation === "suffix")
                  .map((g) => (
                    <option key={g.id} value={g.id}>
                      {label(g)}
                    </option>
                  ))}
              </optgroup>
            </select>
          </label>

          {group && (
            <table className="pv-d-tiers">
              <thead>
                <tr>
                  <th />
                  <th>tier</th>
                  <th>rolls</th>
                  <th>ilvl</th>
                  <th>1 in</th>
                  <th>median</th>
                </tr>
              </thead>
              <tbody>
                {group.tiers.map((tier) => {
                  const row = odds(group, tier.tier, ilvl);
                  const on = tier.tier === status.tierThreshold;
                  return (
                    <tr
                      key={tier.tier}
                      className={on ? "pv-d-tier-on" : ""}
                      onClick={() => setTarget(status.targetGroup, tier.tier)}
                    >
                      <td>
                        <input type="radio" checked={on} readOnly />
                      </td>
                      <td>T{tier.tier}</td>
                      <td className="pv-mono">{bands(group, tier.tier)}</td>
                      <td className="pv-mono">{tier.requiredIlvl}</td>
                      <td className="pv-mono">
                        {row.impossible ? "—" : row.oneIn}
                      </td>
                      <td className="pv-mono">{row.impossible ? "—" : row.median}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}

          <label className="pv-d-ilvl">
            <span>jewel item level</span>
            <input
              type="number"
              min={1}
              max={100}
              value={ilvl}
              onChange={(event) => setIlvl(Number(event.target.value))}
            />
          </label>
          <p className="pv-d-note">
            Odds are per click, not per prefix. The one-or-two-affix split behind them is
            community-derived rather than from GGG data, so treat the last digit as approximate.
          </p>
        </div>
      )}

      {/* The ledger. Newest first so the thing that just happened needs no scrolling. */}
      <div className="pv-d-ledger">
        {logLines.length === 0 ? (
          <p className="pv-muted">Nothing logged yet.</p>
        ) : (
          [...logLines].reverse().map((line, index) => (
            <div key={`${index}-${line.slice(0, 24)}`} className={`pv-d-line pv-d-${kindOf(line)}`}>
              {line}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
