/**
 * The Craft Session panel: choose a Target Mod, arm, and watch what the cycle says.
 *
 * Two things it deliberately does *not* do. **It never decides anything** — every Verdict, Refusal and
 * Halt is `poe-graft-core`'s, and every explanatory sentence it renders is Rust's own `message`, so
 * the window and the log file cannot disagree. And **it never paraphrases an error**: on a machine with
 * no dev tools the raw string is the only diagnostic there is.
 *
 * This is not a design. [#9](https://github.com/Furizaa/poe-graft/issues/9) owns what the window should
 * look like and which sounds it makes; what is here is the smallest thing that lets the map's
 * acceptance test happen — roll a Ghastly Eye Jewel by tapping the Trigger Key, and have the app
 * decline to roll again on the Hit.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  acknowledge,
  getCycleStatus,
  getModPool,
  getModPoolSource,
  note,
  setArmed,
  setTarget,
  setTrigger,
  type CycleStatus,
  type ModGroup,
  type ModPool,
} from "./api";
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

/** Every state a Craft Session can be in, and how loudly to say so. */
const TONE: Record<CycleStatus["state"], string> = {
  Idle: "",
  Sighting: "good",
  Ready: "good",
  Rolling: "good",
  Resyncing: "bad",
  Latched: "good",
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

export default function Craft({ refreshLog }: { refreshLog: () => Promise<void> }) {
  const [pool, setPool] = useState<ModPool | null>(null);
  const [poolError, setPoolError] = useState<string | null>(null);
  const [poolSource, setPoolSource] = useState("");
  const [status, setStatus] = useState<CycleStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [observation, setObservation] = useState("");
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

  const group = useMemo(
    () => pool?.groups.find((g) => g.id === status?.targetGroup) ?? null,
    [pool, status?.targetGroup],
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
  const prefixes = pool.groups.filter((g) => g.generation === "prefix");
  const suffixes = pool.groups.filter((g) => g.generation === "suffix");

  return (
    <section>
      <h2>Craft session</h2>

      {/* The badge, and core's own sentence about why. After a Halt this is the only thing that says
          what went wrong, so it is rendered verbatim and given the most room on the panel. */}
      <div className="row">
        <span className={`state ${TONE[status.state]}`}>{status.state}</span>
        <span className="badge">
          tap <strong>{status.triggerName}</strong>
        </span>
        {armed && <span className="badge good">trigger suppressed</span>}
        <span className="badge">{status.rolls} Alterations</span>
        {status.lastVerdict && (
          <span className="badge">
            last Read {status.lastVerdict}
            {status.lastTier !== null ? ` · Tier ${status.lastTier}` : ""}
          </span>
        )}
      </div>
      <p className={status.state === "Halted" ? "error" : ""}>{status.message}</p>

      {status.state === "Latched" && (
        <div className="row">
          <button className="primary" onClick={() => void act(acknowledge)}>
            Acknowledge the Hit
          </button>
          <span className="muted small">
            Take the orb off your cursor first. This opens a new Craft Session on the same Target Mod
            — hover the same jewel and it will simply Latch again rather than roll it.
          </span>
        </div>
      )}

      {status.state === "Halted" && (
        <div className="row">
          <button className="primary" onClick={() => void act(() => setArmed(true))}>
            Re-arm
          </button>
        </div>
      )}

      {/* Step 1. The target, before anything is armed — it cannot change afterwards, because the
          session would be holding a Verdict about the old one. */}
      <h3>1 · Target Mod</h3>
      <div className="row">
        <label className="field grow">
          <span className="muted small">Mod Group</span>
          <select
            disabled={armed}
            value={status.targetGroup}
            onChange={(event) => {
              const id = event.target.value;
              const next = pool.groups.find((g) => g.id === id);
              void act(() =>
                setTarget(id, next ? carryThreshold(next, status.tierThreshold) : 1),
              );
            }}
          >
            <optgroup label="Prefixes">
              {prefixes.map((g) => (
                <option key={g.id} value={g.id}>
                  {label(g)}
                </option>
              ))}
            </optgroup>
            <optgroup label="Suffixes">
              {suffixes.map((g) => (
                <option key={g.id} value={g.id}>
                  {label(g)}
                </option>
              ))}
            </optgroup>
          </select>
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
      </div>
      <p className="muted small">
        {group
          ? `A Hit is Tier ${status.tierThreshold} or better — ${bands(group, status.tierThreshold)}. Tier is derived from the numbers the game prints, never from its own tier annotation.`
          : "Choose a Mod Group."}
      </p>
      {group && group.tiers.some((t) => t.tier === status.tierThreshold && t.requiredIlvl > 0) && (
        <p className="muted small">
          The jewel must be item level{" "}
          <span className="mono">
            {group.tiers.find((t) => t.tier === status.tierThreshold)?.requiredIlvl}
          </span>{" "}
          or higher, or this tier cannot spawn at all.
        </p>
      )}

      {/* Step 2. Sticky Keys is checked first because it breaks Apply Mode with no error and no
          visible sign, which would make every Refusal look like a bug in this app. */}
      <h3>2 · Environment</h3>
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
      <p className="muted small mono">foreground {status.foreground}</p>

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
          Typed rather than learned by pressing, because the hook is deaf while this window has focus
          (<a href="https://github.com/Furizaa/poe-graft/issues/18">#18</a>). Default{" "}
          <span className="mono">219</span> is <span className="mono">[</span> — the only key with
          on-device evidence behind it. Scroll Lock <span className="mono">145</span> · Pause{" "}
          <span className="mono">19</span> · Insert <span className="mono">45</span> · Home{" "}
          <span className="mono">36</span> · Numpad 0 <span className="mono">96</span>
        </span>
      </div>

      {/* Step 3. Arming is a mouse click in this window, so the hook is deaf at the moment it
          happens — which is why core's `Sighting` message says "click into Path of Exile" and why
          that sentence is load-bearing rather than polish. */}
      <h3>3 · Run</h3>
      <div className="row">
        <button
          className={armed ? "" : "primary"}
          disabled={!status.supported || !status.running || status.state === "Latched"}
          onClick={() => {
            enableSounds();
            void act(() => setArmed(!armed));
          }}
        >
          {armed ? "Stop" : "Arm"}
        </button>
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
      {status.state === "Latched" && (
        <p className="muted small">
          Stop is disabled while a Hit is Latched — acknowledge it instead, so a misclick cannot throw
          the Hit away.
        </p>
      )}
      {!status.supported && (
        <p className="muted small">
          The roll cycle only runs on Windows. On the development machine there is no hook, no game and
          no clipboard worth poisoning — this panel is here so the layout and the Target Mod picker can
          be checked, and it reports real numbers on the gaming PC.
        </p>
      )}
      {status.pressesDropped > 0 && (
        <p className="muted small">
          Dropped presses are fail-closed sequencing refusing to queue work, not a bug: a queued press
          would be a second Alteration nobody asked for.
        </p>
      )}

      {/* The human's own observations belong in the same file, in order, timestamped — otherwise
          "Apply Mode dropped out around Roll 30" has to be reconstructed from memory later. */}
      <h3>4 · Note what you saw</h3>
      <div className="row">
        <input
          className="grow"
          value={observation}
          placeholder="e.g. Apply Mode dropped out at Roll 31"
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

      <p className="muted small mono">tier data {poolSource}</p>
      {error && <p className="error">{error}</p>}
    </section>
  );
}
