/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * The prototype chrome: force any state, break the environment on purpose, and audition the sounds.
 *
 * **None of this is part of any variant.** It is deliberately ugly and deliberately labelled SIMULATED,
 * because the one thing that must not happen is mistaking a replay on the Mac for a run on the gaming
 * PC. Nothing here can click, and `supported` stays `false` throughout — the stub keeps telling the
 * truth about injection.
 */
import { drivableStates, type Knobs } from "./mock";
import { benchPlay, soundSets } from "./soundBench";
import { captures } from "./fixtures";

export function Driver({
  knobs,
  setKnobs,
  open,
  setOpen,
}: {
  knobs: Knobs;
  setKnobs: (next: Knobs) => void;
  open: boolean;
  setOpen: (open: boolean) => void;
}) {
  const set = <K extends keyof Knobs>(key: K, value: Knobs[K]) =>
    setKnobs({ ...knobs, [key]: value });

  if (!open) {
    return (
      <button className="pb-tab" onClick={() => setOpen(true)}>
        SIMULATED · {knobs.state} · open driver
      </button>
    );
  }

  return (
    <div className="pb-driver">
      <div className="pb-driver-head">
        <strong>SIMULATED</strong>
        <span>
          No hook, no game, no clipboard. This drives a fake status so every variant can be seen in
          every state on a Mac — it is not evidence about the device.
        </span>
        <button onClick={() => setOpen(false)}>hide</button>
      </div>

      <div className="pb-group">
        <span className="pb-label">State</span>
        {drivableStates.map((state) => (
          <button
            key={state}
            className={knobs.state === state ? "pb-on" : ""}
            onClick={() =>
              setKnobs({
                ...knobs,
                state,
                // Plausible companions, so a forced state is internally consistent.
                lastTier: state === "Latched" ? knobs.tierThreshold : state === "Ready" ? 4 : null,
                consecutiveUnknown: state === "Resyncing" ? 1 : 0,
              })
            }
          >
            {state}
          </button>
        ))}
      </div>

      <div className="pb-group">
        <span className="pb-label">Rolls</span>
        {[0, 1, 26, 193, 512].map((n) => (
          <button key={n} className={knobs.rolls === n ? "pb-on" : ""} onClick={() => set("rolls", n)}>
            {n}
          </button>
        ))}
        <span className="pb-label">Unknown run</span>
        {[1, 2].map((n) => (
          <button
            key={n}
            className={knobs.consecutiveUnknown === n ? "pb-on" : ""}
            onClick={() => set("consecutiveUnknown", n)}
          >
            {n} of 3
          </button>
        ))}
      </div>

      <div className="pb-group">
        <span className="pb-label">Break it</span>
        <button className={knobs.stickyKeys ? "pb-on" : ""} onClick={() => set("stickyKeys", !knobs.stickyKeys)}>
          Sticky Keys
        </button>
        <button className={knobs.hookDeaf ? "pb-on" : ""} onClick={() => set("hookDeaf", !knobs.hookDeaf)}>
          hook deaf
        </button>
        <button className={!knobs.shiftDown ? "pb-on" : ""} onClick={() => set("shiftDown", !knobs.shiftDown)}>
          Shift up
        </button>
        <button className={!knobs.inGame ? "pb-on" : ""} onClick={() => set("inGame", !knobs.inGame)}>
          game not foreground
        </button>
      </div>

      <div className="pb-group">
        <span className="pb-label">Last Read</span>
        {captures.map((capture, index) => (
          <button
            key={capture.name}
            className={knobs.captureIndex === index ? "pb-on" : ""}
            onClick={() => set("captureIndex", index)}
            title={capture.note}
          >
            {capture.name}
          </button>
        ))}
      </div>

      {/* The sound bench. #9 owes an answer on sounds and `src/sounds.ts` is a placeholder. Audition
          with Path of Exile running — a quiet room makes all three sound fine. */}
      <div className="pb-bench">
        <div className="pb-label">
          Sound bench — audition over a running game, not in a quiet room
        </div>
        {soundSets.map((set_) => (
          <div key={set_.key} className="pb-bench-row">
            <div className="pb-bench-name">
              <strong>{set_.name}</strong>
              <span>{set_.argument}</span>
            </div>
            <div className="pb-bench-buttons">
              <button className="pb-hit" onClick={() => benchPlay(set_.hit)}>
                Hit
              </button>
              <button onClick={() => benchPlay(set_.halt)}>Halt</button>
              <button onClick={() => benchPlay(set_.blip)}>blip</button>
              <button
                title="Blip then Hit, back to back — the confusion that actually matters"
                onClick={() => {
                  benchPlay(set_.blip);
                  window.setTimeout(() => benchPlay(set_.hit), 700);
                }}
              >
                blip → Hit
              </button>
            </div>
          </div>
        ))}
        <p className="pb-bench-note">
          The shipped placeholders put the blip at 1500 Hz and the Hit at 880→1320→1760 Hz, so the blip
          sits inside the Hit's range and differs only by being shorter and quieter — the least reliable
          difference there is over game audio. Bell and Alarm both move the blip low instead. Use
          <em> blip → Hit</em> to hear whether that matters.
        </p>
      </div>
    </div>
  );
}
