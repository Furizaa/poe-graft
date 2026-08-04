/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * # C · Glanceable — "the human is looking at the game, not at this"
 *
 * One centred card. The state word is enormous, there are exactly two numbers, and everything else is
 * behind a single disclosure. The premise is that during a Craft Session the window is on a second
 * monitor or behind the game, and the only thing it has to do is survive a one-second glance from
 * across a desk — so anything that cannot be read in one second is not just noise, it is actively
 * crowding out the thing that can.
 *
 * This is the most aggressive answer to "walls of text and debug stuff", and the point of building it
 * is to find where it goes too far. The likely failure is `Halted`: a state whose entire purpose is to
 * explain itself does not fit in a design that refuses to show sentences, which is why the checklist
 * is the one thing allowed to break the rule.
 *
 * ## Threshold control: minimum roll, which is #9's open question
 *
 * Instead of "Tier 1 or better" this asks for a number — *"at least 23 physical damage"* — because that
 * is how the wanted outcome is actually held in someone's head. It resolves to a tier and **says which
 * one**, because the seam is `Target { group_id, tier_threshold }` and the Verdict is tier-based.
 *
 * Two honest limits, both on screen rather than in this comment only:
 *
 * - A number resolves to *the worst tier that guarantees it*. A lucky roll in a lower tier that
 *   happens to reach the number is **not** a Hit, because tier is what the Verdict tests. So this
 *   control is a nicer way to *say* a tier, not a different rule.
 * - On a mod with two rolled values, "at least N" is ambiguous. It is read against the first value.
 */
import { cumulative, label, odds, percent } from "../data";
import { headline, steps, subhead, tone } from "../proposedCopy";
import type { ModGroup } from "../../api";
import type { VariantProps } from "./types";

export const name = "Glanceable — one card, two numbers, minimum-roll target";

/** The lowest value the first band of a tier can produce. */
const floorOf = (group: ModGroup, tier: number) =>
  group.tiers.find((t) => t.tier === tier)?.bands[0]?.[0] ?? 0;

/** The worst tier that *guarantees* at least `min` on the first value. */
const tierForMinimum = (group: ModGroup, min: number) => {
  const eligible = group.tiers.filter((t) => floorOf(group, t.tier) >= min);
  return eligible.length > 0 ? Math.max(...eligible.map((t) => t.tier)) : 1;
};

export default function VariantC({
  status,
  pool,
  group,
  ilvl,
  lastRead,
  setTarget,
  setArmed,
  acknowledge,
}: VariantProps) {
  const armed = status.state !== "Idle";
  const chance = group ? odds(group, status.tierThreshold, ilvl) : null;
  const minimum = group ? floorOf(group, status.tierThreshold) : 0;
  const list = steps(status);

  return (
    <div className="pv-c">
      <div className={`pv-c-card pv-tone-${tone(status)}`}>
        <div className="pv-c-word">{headline(status)}</div>
        <div className="pv-c-sub">{subhead(status)}</div>

        {armed && (
          <div className="pv-c-two">
            <div>
              <strong>{status.rolls}</strong>
              <span>rolls</span>
            </div>
            {chance && !chance.impossible && (
              <div>
                <strong>{percent(cumulative(chance.perClick, status.rolls), 0)}</strong>
                <span>should have hit by now</span>
              </div>
            )}
          </div>
        )}

        {/* The one place this design is allowed to show a list: a Halt exists in order to be read. */}
        {list.length > 0 && status.state === "Halted" && (
          <ul className="pv-c-checks">
            {list.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ul>
        )}

        <div className="pv-c-act">
          {status.state === "Latched" ? (
            <button className="pv-btn pv-btn-hit pv-btn-big" onClick={acknowledge}>
              Got it
            </button>
          ) : status.state === "Halted" ? (
            <button className="pv-btn pv-btn-primary pv-btn-big" onClick={() => setArmed(true)}>
              Re-arm
            </button>
          ) : armed ? (
            <button className="pv-btn" onClick={() => setArmed(false)}>
              Stop
            </button>
          ) : (
            <button
              className="pv-btn pv-btn-primary pv-btn-big"
              disabled={!group}
              onClick={() => setArmed(true)}
            >
              Arm
            </button>
          )}
        </div>
      </div>

      {/* Setup, when not armed. Still small, still centred, still one thing at a time. */}
      {!armed && (
        <div className="pv-c-setup">
          <select
            className="pv-c-group"
            value={status.targetGroup}
            onChange={(event) => setTarget(event.target.value, 1)}
          >
            {pool.groups.map((g) => (
              <option key={g.id} value={g.id}>
                {g.generation === "prefix" ? "prefix" : "suffix"} — {label(g)}
              </option>
            ))}
          </select>

          {group && (
            <>
              <div className="pv-c-min">
                <span>at least</span>
                <input
                  type="number"
                  min={0}
                  value={minimum}
                  onChange={(event) =>
                    setTarget(status.targetGroup, tierForMinimum(group, Number(event.target.value)))
                  }
                />
                <span>on the first value</span>
              </div>
              <p className="pv-c-resolve">
                → <strong>Tier {status.tierThreshold} or better</strong>
                {chance && !chance.impossible ? (
                  <>
                    , about 1 in <strong>{chance.oneIn}</strong> rolls, median{" "}
                    <strong>{chance.median}</strong>
                  </>
                ) : (
                  <span className="pv-warn"> — impossible at item level {ilvl}</span>
                )}
              </p>
              <p className="pv-c-caveat">
                A number is a nicer way to say a tier, not a different rule: poe-graft tests the tier,
                so a lucky lower-tier roll that reaches {minimum || "this"} does not count as a Hit.
              </p>
            </>
          )}
        </div>
      )}

      <details className="pv-c-more">
        <summary>Everything else</summary>
        <div className="pv-c-grid">
          <span>state</span>
          <span className="pv-mono">{status.state}</span>
          <span>trigger</span>
          <span className="pv-mono">
            {status.triggerName} ({status.triggerVk})
          </span>
          <span>hook</span>
          <span className="pv-mono">
            {status.running ? "installed" : "not installed"} · {status.keysSeen} keys
          </span>
          <span>shift</span>
          <span className="pv-mono">{status.shiftDown ? "held" : "up"}</span>
          <span>sticky keys</span>
          <span className="pv-mono">{status.accessibility?.stickyKeysOn ? "ON" : "off"}</span>
          <span>anchor</span>
          <span className="pv-mono">
            {status.anchor ? `${status.anchor[0]},${status.anchor[1]}` : "none"}
          </span>
          <span>presses</span>
          <span className="pv-mono">
            {status.presses} · {status.pressesDropped} dropped
          </span>
          <span>timing</span>
          <span className="pv-mono">
            cycle {status.cycleMs ?? "–"}ms · copy {status.copyMs ?? "–"}ms
          </span>
          <span>foreground</span>
          <span className="pv-mono">{status.foreground}</span>
          <span>logged</span>
          <span className="pv-small">{status.message}</span>
        </div>
        {lastRead && <pre>{lastRead}</pre>}
      </details>
    </div>
  );
}
