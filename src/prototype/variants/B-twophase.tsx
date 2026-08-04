/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * # B · Two-phase — "mid-craft the window should offer almost nothing"
 *
 * Setup and session are different screens, and arming **replaces** one with the other. During a Craft
 * Session there is no picker, no trigger-key field, no hook counter and no timing readout, because
 * none of them can be acted on while the cursor has to stay on the jewel — the settings are already
 * unusable mid-craft in the shipped window, they are just still *visible*, which is most of what makes
 * it feel dense.
 *
 * The setup screen absorbs the debug in the only form it is actually useful: a **preflight checklist**
 * that has to go green before Arm unlocks. Today those facts are badges the human is expected to
 * interpret — `0 keystrokes seen`, `Sticky Keys ON` — with the interpretation written in a paragraph
 * underneath. Here each one is a check with a fix attached, and it disappears once it passes.
 *
 * Threshold control: a **slider**, with the numeric bands and the odds updating live underneath, so
 * the cost of asking for one tier better is visible while you ask for it rather than after.
 *
 * Uses `proposedCopy` — so a win here means changing `crates/core`'s strings.
 */
import { useState } from "react";
import { bands, cumulative, label, odds, percent } from "../data";
import { headline, steps, subhead, tone } from "../proposedCopy";
import type { VariantProps } from "./types";

export const name = "Two-phase — setup, then a session screen that hides everything";

export default function VariantB({
  status,
  pool,
  group,
  ilvl,
  setIlvl,
  setTarget,
  setArmed,
  acknowledge,
}: VariantProps) {
  const [filter, setFilter] = useState("");
  const armed = status.state !== "Idle";
  const chance = group ? odds(group, status.tierThreshold, ilvl) : null;

  // ── Session screen ──────────────────────────────────────────────────────────────────────────
  if (armed) {
    const list = steps(status);
    return (
      <div className={`pv-b-session pv-tone-${tone(status)}`}>
        <div className="pv-b-headline">{headline(status)}</div>
        <p className="pv-b-sub">{subhead(status)}</p>

        {list.length > 0 && (
          <ol className="pv-b-steps">
            {list.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        )}

        <div className="pv-b-count">
          <div>
            <strong>{status.rolls}</strong>
            <span>alterations spent</span>
          </div>
          {chance && !chance.impossible && (
            <>
              <div>
                <strong>{percent(cumulative(chance.perClick, status.rolls), 1)}</strong>
                <span>chance you'd have hit by now</span>
              </div>
              <div>
                <strong>{chance.median}</strong>
                <span>median rolls for this target</span>
              </div>
            </>
          )}
        </div>

        <div className="pv-b-controls">
          {status.state === "Latched" ? (
            <button className="pv-btn pv-btn-hit pv-btn-big" onClick={acknowledge}>
              Acknowledge the Hit
            </button>
          ) : status.state === "Halted" ? (
            <button className="pv-btn pv-btn-primary pv-btn-big" onClick={() => setArmed(true)}>
              Re-arm
            </button>
          ) : (
            <button className="pv-btn" onClick={() => setArmed(false)}>
              Stop
            </button>
          )}
        </div>

        {/* Core's own sentence is still reachable, and is still what went to the log — it is just no
            longer the first thing you read. One fold, and nothing else. */}
        <details className="pv-b-verbatim">
          <summary>What poe-graft logged</summary>
          <p>{status.message}</p>
        </details>
      </div>
    );
  }

  // ── Setup screen ────────────────────────────────────────────────────────────────────────────
  const matches = pool.groups.filter((g) =>
    filter.trim() ? label(g).toLowerCase().includes(filter.trim().toLowerCase()) : true,
  );
  const prefixes = matches.filter((g) => g.generation === "prefix");
  const suffixes = matches.filter((g) => g.generation === "suffix");

  const checks = [
    {
      ok: status.running,
      label: "Keyboard hook installed",
      fix: "The hook is what suppresses the trigger key. Without it nothing can be armed.",
    },
    {
      ok: status.keysSeen > 0,
      label: "The hook is hearing keys",
      fix: "Installed but deaf. Press any key — if this stays at zero, no amount of pressing will help.",
    },
    {
      ok: !status.accessibility?.stickyKeysOn,
      label: "Sticky Keys is off",
      fix: "It changes what holding Shift means with no visible sign, so Apply Mode drops out mid-craft. Settings → Accessibility → Keyboard.",
    },
    {
      ok: !!group && !chance?.impossible,
      label: "This tier can spawn on this jewel",
      fix: `No tier at or better than your threshold can spawn at item level ${ilvl}.`,
    },
  ];
  const blocking = checks.filter((c) => !c.ok);

  const maxTier = group ? Math.max(...group.tiers.map((t) => t.tier)) : 1;

  return (
    <div className="pv-b-setup">
      <h2 className="pv-b-h">What are you looking for?</h2>

      <input
        className="pv-b-filter"
        placeholder={`Filter ${pool.groups.length} mod groups — try "Life" or "Physical"`}
        value={filter}
        onChange={(event) => setFilter(event.target.value)}
      />

      <div className="pv-b-cols">
        <div className="pv-b-col">
          <h3>
            Prefixes <span>{prefixes.length}</span>
          </h3>
          <div className="pv-b-list">
            {prefixes.map((g) => (
              <button
                key={g.id}
                className={g.id === status.targetGroup ? "pv-b-row pv-b-row-on" : "pv-b-row"}
                onClick={() => setTarget(g.id, Math.min(status.tierThreshold, g.tiers.length))}
              >
                {label(g)}
              </button>
            ))}
          </div>
        </div>
        <div className="pv-b-col">
          <h3>
            Suffixes <span>{suffixes.length}</span>
          </h3>
          <div className="pv-b-list">
            {suffixes.map((g) => (
              <button
                key={g.id}
                className={g.id === status.targetGroup ? "pv-b-row pv-b-row-on" : "pv-b-row"}
                onClick={() => setTarget(g.id, Math.min(status.tierThreshold, g.tiers.length))}
              >
                {label(g)}
              </button>
            ))}
          </div>
        </div>
      </div>

      {group && (
        <div className="pv-b-tier">
          <div className="pv-b-tier-head">
            <span>How good does it have to be?</span>
            <strong>
              Tier {status.tierThreshold} or better
              {maxTier === 1 ? " — this group has only one tier" : ""}
            </strong>
          </div>
          <input
            type="range"
            min={1}
            max={maxTier}
            step={1}
            value={status.tierThreshold}
            disabled={maxTier === 1}
            onChange={(event) => setTarget(status.targetGroup, Number(event.target.value))}
          />
          <div className="pv-b-tier-live">
            <span>
              Accepts <strong>{bands(group, status.tierThreshold)}</strong>
            </span>
            {chance && !chance.impossible ? (
              <>
                <span>
                  1 in <strong>{chance.oneIn}</strong> rolls
                </span>
                <span>
                  median <strong>{chance.median}</strong>
                </span>
              </>
            ) : (
              <span className="pv-warn">impossible at item level {ilvl}</span>
            )}
          </div>
          <label className="pv-b-ilvl">
            <span>Jewel item level</span>
            <input
              type="number"
              min={1}
              max={100}
              value={ilvl}
              onChange={(event) => setIlvl(Number(event.target.value))}
            />
            <span className="pv-muted pv-small">
              The odds move with it, and poe-graft can only read it once you have armed.
            </span>
          </label>
        </div>
      )}

      <div className="pv-b-preflight">
        {blocking.length === 0 ? (
          <p className="pv-ok">Everything checks out.</p>
        ) : (
          blocking.map((check) => (
            <div key={check.label} className="pv-b-check">
              <strong>{check.label}</strong>
              <span>{check.fix}</span>
            </div>
          ))
        )}
      </div>

      <button
        className="pv-btn pv-btn-primary pv-btn-big"
        disabled={blocking.length > 0}
        onClick={() => setArmed(true)}
      >
        Arm
      </button>
    </div>
  );
}
