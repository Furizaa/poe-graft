/**
 * The Craft Session panel: choose a Target Mod, arm, and watch what the cycle says.
 *
 * Two things it deliberately does *not* do. **It never decides anything** — every Verdict, Refusal and
 * Halt is `poe-graft-core`'s, and every explanatory sentence it renders is Rust's own `message`, so
 * the window and the log file cannot disagree. And **it never paraphrases an error**: on a machine with
 * no dev tools the raw string is the only diagnostic there is.
 *
 * ## What [#9](https://github.com/Furizaa/poe-graft/issues/9) settled
 *
 * The first version of this panel was an instrument panel — four numbered steps of badges, with the hook
 * counters, the accessibility flags and the timings all given equal weight. It worked, and the verdict on
 * using it was *"walls of text and debug stuff"*. Four candidate layouts were prototyped on
 * `prototype/craft-ui`; this is the one that won, and the shape of it is the decision:
 *
 * - **One thing is the biggest thing on screen** — the state — and core's sentence sits directly under it.
 * - **The primary control is next to the sentence that asks for it**, never further down the page.
 * - **The target is a sentence**, with the picker underneath rather than in place of it.
 * - **Every diagnostic is in one collapsed strip at the bottom.** Nothing is removed: on a machine with
 *   no dev tools this is the entire diagnostic surface, so it stays reachable and grows a red dot when
 *   something in it needs reading. It opens itself on a Halt.
 *
 * The copy is untouched — this layout changes no `message` string, which is what made it the honest
 * comparison against the old panel. Whether core's `Sighting` and `Halted` paragraphs should be shortened
 * is still open on #9.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  acknowledge,
  cumulative,
  getCycleStatus,
  getModOdds,
  getModPool,
  getModPoolSource,
  note,
  setArmed,
  setTarget,
  setTrigger,
  type CycleStatus,
  type ModGroup,
  type ModPool,
  type Odds,
} from "./api";
import ModPicker from "./ModPicker";
import { enableSounds, playBlip, playHalt, playHit } from "./sounds";

const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

/**
 * How often to re-poll.
 *
 * Fast enough that the Hit sound lands while the human's hand is still on the key, cheap enough to
 * ignore. Safety does not depend on it: the Latch refuses the next press whether or not the window has
 * noticed yet.
 */
const POLL_MS = 100;

/**
 * The item level the odds are computed against, before a session can read the real one.
 *
 * The odds move with item level — the map's target is 1 in 272 on an ilvl 83 jewel and 1 in 278 at 86 or
 * above — so a number has to be assumed until an Item Text arrives. 83 is the default because that is the
 * level T1 of the acceptance-test target requires, and it is what the captures are.
 */
const DEFAULT_ILVL = 83;

/** Every state a Craft Session can be in, and how loudly to say so. */
const TONE: Record<CycleStatus["state"], string> = {
  Idle: "",
  Sighting: "good",
  Ready: "good",
  Rolling: "good",
  Resyncing: "bad",
  Latched: "hit",
  Halted: "bad",
};

/** `Minions deal # to # additional Physical Damage` — never an affix name, which is per tier. */
const label = (group: ModGroup) => group.lines.join(" · ");

/** `23–26 to 33–39`, the numbers the game will print. */
const bands = (group: ModGroup, tier: number) => {
  const found = group.tiers.find((t) => t.tier === tier);
  if (!found) return "";
  return found.bands.map(([min, max]) => (min === max ? `${min}` : `${min}–${max}`)).join(" to ");
};

/**
 * The Tier Threshold to carry across a change of Mod Group.
 *
 * Groups do not all have the same number of tiers — **40 of this Base's 66 have only a Tier 1** — and
 * Rust rejects a Target Mod naming a tier its group does not have. Sending the old threshold
 * unchanged therefore makes most group changes fail outright: the select snaps back on the next poll
 * and the human is told "no Tier 5", which reads like a data fault rather than the picker's own.
 *
 * Clamping to the worst tier the new group has preserves the intent, because the threshold means
 * "this tier or better": on a group whose only tier is 1, "Tier 1 or better" *is* "any tier of it".
 */
const carryThreshold = (group: ModGroup, threshold: number) =>
  Math.min(threshold, Math.max(...group.tiers.map((t) => t.tier)));

/** `0.37%` — three decimals under one percent, because two would round the interesting ones to 0.00%. */
const percent = (value: number, places = 1) =>
  `${(value * 100).toFixed(value * 100 < 1 ? 3 : places)}%`;

export default function Craft({ refreshLog }: { refreshLog: () => Promise<void> }) {
  const [pool, setPool] = useState<ModPool | null>(null);
  const [poolError, setPoolError] = useState<string | null>(null);
  const [poolSource, setPoolSource] = useState("");
  const [status, setStatus] = useState<CycleStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [observation, setObservation] = useState("");
  const [picking, setPicking] = useState(false);
  const [ilvl, setIlvl] = useState(DEFAULT_ILVL);
  const [odds, setOdds] = useState<Odds | null>(null);
  /** The Trigger Key code being typed, so the field does not fight the poll. */
  const [triggerDraft, setTriggerDraft] = useState<number | null>(null);

  /** Feedback counters last seen, so each increment sounds exactly once. */
  const heard = useRef({ hits: 0, halts: 0, blips: 0, first: true });
  /** Pull the log forward when something happened, rather than on every poll. */
  const lastSeen = useRef({ presses: -1, state: "" });

  useEffect(() => {
    void (async () => {
      try {
        setPool(await getModPool());
      } catch (caught) {
        setPoolError(errorText(caught));
      }
      setPoolSource(await getModPoolSource());
    })();
  }, []);

  useEffect(() => {
    let live = true;
    const poll = async () => {
      try {
        const next = await getCycleStatus();
        if (!live) return;
        setStatus(next);

        // One sound per increment. The first poll adopts whatever the counters already are, so
        // reloading the window does not replay a Hit from ten minutes ago.
        const seen = heard.current;
        if (seen.first) {
          heard.current = { hits: next.hits, halts: next.halts, blips: next.blips, first: false };
        } else {
          if (next.hits > seen.hits) playHit();
          else if (next.halts > seen.halts) playHalt();
          else if (next.blips > seen.blips) playBlip();
          heard.current = { hits: next.hits, halts: next.halts, blips: next.blips, first: false };
        }

        if (next.presses !== lastSeen.current.presses || next.state !== lastSeen.current.state) {
          lastSeen.current = { presses: next.presses, state: next.state };
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

  const group = useMemo(
    () => pool?.groups.find((g) => g.id === status?.targetGroup) ?? null,
    [pool, status?.targetGroup],
  );

  // Odds are Rust's, so they are fetched rather than derived — on a change of target, threshold or item
  // level, and not on every poll. `null` covers both "no weights in the data" and "no target yet", which
  // render the same way: no numbers rather than wrong ones.
  const targetGroup = status?.targetGroup;
  const tierThreshold = status?.tierThreshold;
  useEffect(() => {
    if (!targetGroup || tierThreshold === undefined) {
      setOdds(null);
      return;
    }
    let live = true;
    void getModOdds(targetGroup, tierThreshold, ilvl)
      .then((next) => live && setOdds(next))
      .catch(() => live && setOdds(null));
    return () => {
      live = false;
    };
  }, [targetGroup, tierThreshold, ilvl]);

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

  if (poolError) {
    return (
      <section>
        <h2>Craft session</h2>
        <p className="error">
          The tier data did not load, so poe-graft cannot assess a Read and will not arm:{" "}
          {poolError}
        </p>
        <p className="muted small">
          There is no fallback tier table and there must not be one — guessing at the numbers is
          exactly how an over-roll gets through. Check <span className="mono">bundle.resources</span>{" "}
          in <span className="mono">tauri.conf.json</span>.
        </p>
      </section>
    );
  }

  if (!status || !pool) {
    return (
      <section>
        <h2>Craft session</h2>
        <p className="muted">Loading…</p>
      </section>
    );
  }

  const armed = status.state !== "Idle";
  const accessibility = status.accessibility;
  const showOdds = odds !== null && !odds.impossible;
  const needsAttention =
    !status.running || status.keysSeen === 0 || accessibility?.stickyKeysOn === true;

  return (
    <section className="craft">
      {/* The state, and core's own sentence about why. After a Halt this is the only thing that says
          what went wrong, so it is rendered verbatim and given the top of the panel. */}
      <div className={`craft-band ${TONE[status.state]}`}>
        <div className="craft-state">{status.state}</div>
        <p className="craft-message">{status.message}</p>
      </div>

      {/* The control the state is asking for, immediately under the sentence asking for it. */}
      <div className="craft-act">
        {status.state === "Latched" ? (
          <button
            className="primary hit"
            onClick={() => {
              // Every button that starts or continues a craft re-enables sound, not just Arm: a user
              // gesture is the only thing guaranteed to resume a context the webview suspended.
              enableSounds();
              void act(acknowledge);
            }}
          >
            Acknowledge the Hit
          </button>
        ) : status.state === "Halted" ? (
          <button
            className="primary"
            onClick={() => {
              enableSounds();
              void act(() => setArmed(true));
            }}
          >
            Re-arm
          </button>
        ) : (
          <button
            className={armed ? "" : "primary"}
            disabled={!status.supported || !status.running}
            onClick={() => {
              enableSounds();
              void act(() => setArmed(!armed));
            }}
          >
            {armed ? "Stop" : "Arm"}
          </button>
        )}

        <div className="craft-numbers">
          <span>
            <strong>{status.rolls}</strong> Alterations
          </span>
          {showOdds && (
            <span>
              <strong>{percent(cumulative(odds, status.rolls))}</strong> chance by now
            </span>
          )}
          {showOdds && odds.medianRolls !== null && (
            <span>
              median <strong>{odds.medianRolls}</strong>
            </span>
          )}
          {status.lastVerdict && (
            <span>
              last Read <strong>{status.lastVerdict}</strong>
              {status.lastTier !== null ? ` · Tier ${status.lastTier}` : ""}
            </span>
          )}
        </div>
      </div>

      {status.state === "Latched" && (
        <p className="muted small">
          Take the orb off your cursor first. This opens a new Craft Session on the same Target Mod —
          hover the same jewel and it will simply Latch again rather than roll it.
        </p>
      )}

      {/* The target as a sentence, with the controls under it rather than instead of it. It cannot
          change while armed, because the session would be holding a Verdict about the old one. */}
      <div className="craft-target">
        {group ? (
          <p>
            Looking for <strong>{label(group)}</strong> at{" "}
            <strong>Tier {status.tierThreshold}</strong> or better — {bands(group, status.tierThreshold)}.
            {odds?.impossible ? (
              <span className="error">
                {" "}
                No tier that good can spawn on an item level {ilvl} jewel.
              </span>
            ) : showOdds && odds.oneIn !== null ? (
              <>
                {" "}
                About <strong>1 in {odds.oneIn}</strong> rolls.
              </>
            ) : null}
          </p>
        ) : (
          <p className="muted">No Target Mod chosen yet.</p>
        )}

        <div className="row">
          <label className="field grow">
            <span className="muted small">Mod Group</span>
            <button className="pick-open" disabled={armed} onClick={() => setPicking(true)}>
              <span className="pick-open-value">
                {group ? label(group) : "Choose a Mod Group…"}
              </span>
              <span className="muted small">
                {group ? `${group.generation} · ` : ""}search {pool.groups.length}
              </span>
            </button>
          </label>
          <label className="field">
            <span className="muted small">Tier Threshold</span>
            <select
              disabled={armed || !group}
              value={status.tierThreshold}
              onChange={(event) =>
                void act(() => setTarget(status.targetGroup, Number(event.target.value)))
              }
            >
              {group?.tiers.map((tier) => (
                <option key={tier.tier} value={tier.tier}>
                  T{tier.tier} · {bands(group, tier.tier)} · ilvl {tier.requiredIlvl}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span className="muted small">Jewel item level</span>
            <input
              type="number"
              min={1}
              max={100}
              value={ilvl}
              onChange={(event) => {
                const next = Number(event.target.value);
                if (Number.isFinite(next)) setIlvl(next);
              }}
            />
          </label>
        </div>
        <p className="muted small">
          Tier is derived from the numbers the game prints, never from its own tier annotation. The odds
          move with item level, and are per click rather than per prefix — the one-or-two-affix split
          behind them is community-derived rather than from the game's data, so treat the last digit as
          approximate.
        </p>
      </div>

      {picking && (
        <ModPicker
          pool={pool}
          current={status.targetGroup}
          onClose={() => setPicking(false)}
          onPick={(groupId) => {
            const next = pool.groups.find((g) => g.id === groupId);
            void act(() => setTarget(groupId, next ? carryThreshold(next, status.tierThreshold) : 1));
          }}
        />
      )}

      {/* The human's own observations belong in the same file, in order, timestamped — otherwise
          "Apply Mode dropped out around Roll 30" has to be reconstructed from memory later. */}
      <div className="row craft-note">
        <input
          className="grow"
          value={observation}
          placeholder="Note what you saw — e.g. Apply Mode dropped out at Roll 31"
          onChange={(event) => setObservation(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && observation.trim()) {
              const line = observation.trim();
              setObservation("");
              void act(() => note(line));
            }
          }}
        />
        <button
          disabled={!observation.trim()}
          onClick={() => {
            const line = observation.trim();
            setObservation("");
            void act(() => note(line));
          }}
        >
          Log it
        </button>
      </div>

      {error && <p className="error">{error}</p>}

      {/* Everything that is only ever interesting when something is wrong. One fold, at the bottom, shut
          by default — but never removed, because on a machine with no dev tools this is the whole
          diagnostic surface. It opens itself on a Halt, and carries a red dot when something inside it
          needs reading, which is the compromise between out of the way and undiscoverable. */}
      <details className="craft-diag" open={status.state === "Halted"}>
        <summary>
          Diagnostics
          {needsAttention && <span className="dot" />}
        </summary>

        <div className="row">
          <span className={status.running ? "badge good" : "badge bad"}>
            WH_KEYBOARD_LL {status.running ? "installed" : "NOT installed"}
          </span>
          <span
            className={status.running ? (status.keysSeen > 0 ? "badge good" : "badge bad") : "badge"}
          >
            {status.keysSeen} keystrokes seen
          </span>
          <span className={status.shiftDown ? "badge good" : "badge"}>
            Shift {status.shiftDown ? "held" : "up"}
          </span>
          {armed && <span className="badge good">trigger suppressed</span>}
          {accessibility && (
            <>
              <span className={accessibility.stickyKeysOn ? "badge bad" : "badge"}>
                Sticky Keys {accessibility.stickyKeysOn ? "ON" : "off"}
              </span>
              <span className={accessibility.filterKeysOn ? "badge bad" : "badge"}>
                Filter Keys {accessibility.filterKeysOn ? "ON" : "off"}
              </span>
            </>
          )}
          <span className="badge">
            {status.anchor ? `Anchor ${status.anchor[0]},${status.anchor[1]}` : "no Anchor yet"}
          </span>
          <span className="badge">{status.presses} presses</span>
          {status.pressesDropped > 0 && (
            <span className="badge">{status.pressesDropped} dropped mid-cycle</span>
          )}
          {status.consecutiveUnknown > 0 && (
            <span className="badge bad">
              {status.consecutiveUnknown} / {status.unknownLimit} Unknown in a row
            </span>
          )}
          {status.cycleMs !== null && (
            <span className="badge">
              last cycle {status.cycleMs}ms · copy {status.copyMs}ms
            </span>
          )}
        </div>

        {status.running && status.keysSeen === 0 && (
          <p className="error">
            The hook is installed but has not seen a single keystroke. Press any key now — this should
            climb immediately. If it stays at 0 the hook is deaf rather than this window being wrong,
            and no amount of pressing keys will help.
          </p>
        )}
        {accessibility?.stickyKeysOn && (
          <p className="error">
            Sticky Keys is on. It changes what holding Shift means, with no error and no visible sign,
            so Apply Mode will drop out mid-craft. Turn it off in Settings → Accessibility → Keyboard.
          </p>
        )}
        {accessibility?.stickyKeysAvailable && !accessibility.stickyKeysOn && (
          <p className="muted small">
            Sticky Keys is off but its five-taps-on-Shift shortcut is enabled — which a Shift-heavy
            crafting session can trip by accident. Worth disabling the shortcut too.
          </p>
        )}
        {status.pressesDropped > 0 && (
          <p className="muted small">
            Dropped presses are fail-closed sequencing refusing to queue work, not a bug: a queued press
            would be a second Alteration nobody asked for. The newest press to arrive during a cycle is
            served when the cycle ends, so <span className="mono">presses − Alterations − dropped</span>{" "}
            is the number that reconciles.
          </p>
        )}
        {!status.supported && (
          <p className="muted small">
            The roll cycle only runs on Windows. On the development machine there is no hook, no game and
            no clipboard worth poisoning — this panel is here so the layout and the Target Mod picker can
            be checked, and it reports real numbers on the gaming PC.
          </p>
        )}

        <div className="row">
          <label className="field">
            <span className="muted small">Trigger Key code</span>
            <input
              type="number"
              min={0}
              max={255}
              disabled={armed}
              value={triggerDraft ?? status.triggerVk}
              onChange={(event) => {
                const next = Number(event.target.value);
                if (!Number.isFinite(next)) return;
                setTriggerDraft(next);
                // The draft is cleared whether or not Rust accepted it. `act` swallows the rejection
                // into the error line, so clearing it *after* the await left a rejected code — a typed
                // 300, or a stray minus sign — sitting in the field for the rest of the session, with
                // the badge above still showing the key that is really installed.
                void act(() => setTrigger(next)).finally(() => setTriggerDraft(null));
              }}
            />
          </label>
          <span className="muted small">
            Currently <span className="mono">{status.triggerName}</span>. Typed rather than learned by
            pressing, because the hook is deaf while this window has focus (
            <a href="https://github.com/Furizaa/poe-graft/issues/18">#18</a>). Default{" "}
            <span className="mono">219</span> is <span className="mono">[</span> — the only key with
            on-device evidence behind it. Scroll Lock <span className="mono">145</span> · Pause{" "}
            <span className="mono">19</span> · Insert <span className="mono">45</span> · Home{" "}
            <span className="mono">36</span> · Numpad 0 <span className="mono">96</span>
          </span>
        </div>

        <p className="muted small mono">
          foreground {status.foreground} · tier data {poolSource}
        </p>
        {showOdds && (
          <p className="muted small">
            Odds: {percent(odds.perClick)} per click ({percent(odds.conditional)} given a{" "}
            {group?.generation ?? "prefix"}), weight {odds.weight} at item level {ilvl}.
          </p>
        )}
      </details>
    </section>
  );
}
