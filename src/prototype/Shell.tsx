/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * Hosts the variants on the existing craft surface — `UI.md` sub-shape **A**, which it strongly
 * prefers and which is genuinely available here because #20 already shipped a working window.
 *
 * One scope decision, made deliberately and worth arguing with. #9 is titled *"the mod selection and
 * craft session UI"*, which is `Craft.tsx`. But the owner's verdict was about the **window**, and
 * `App.tsx` contributes a "This build" definition list, an Updates panel, a "Platform seam" section
 * with a *Read platform* button, and a raw `<pre>` log dump — wrapped around and below the craft panel.
 * A variant that fixed only the craft section would be judged inside a frame that is itself most of the
 * complaint. So the variants own the whole window, and `live` renders today's `App` untouched for
 * comparison.
 *
 * Gated on `import.meta.env.DEV` at the call site in `main.tsx`, so none of this can reach a build.
 */
import { useMemo, useState } from "react";
import App from "../App";
import type { ModPool } from "../api";
import { getModPool } from "../api";
import { localPool } from "./data";
import { captures } from "./fixtures";
import { defaultKnobs, mockLog, mockStatus, type Knobs } from "./mock";
import { Driver } from "./Driver";
import { Switcher, readVariant, variantKeys, type VariantKey } from "./Switcher";
import VariantA, { name as nameA } from "./variants/A-cockpit";
import VariantB, { name as nameB } from "./variants/B-twophase";
import VariantC, { name as nameC } from "./variants/C-glance";
import VariantD, { name as nameD } from "./variants/D-ledger";
import type { VariantProps } from "./variants/types";
import "./prototype.css";

const labels: Record<VariantKey, string> = {
  live: "today's window, untouched",
  A: nameA,
  B: nameB,
  C: nameC,
  D: nameD,
};

/**
 * The real pool if Tauri is there, the file if it is not.
 *
 * Deliberately synchronous-with-fallback rather than a loading state: the whole point is that the
 * prototype runs in a plain browser, and a spinner that never resolves would hide that it worked.
 */
const usePool = (): ModPool => {
  const [pool, setPool] = useState<ModPool>(localPool);
  useMemo(() => {
    void getModPool()
      .then(setPool)
      .catch(() => {
        // No `invoke()` — running under `pnpm dev` in a browser. The file-derived pool is identical
        // in shape and comes from the same source of truth, so nothing about the picker is faked.
      });
  }, []);
  return pool;
};

/**
 * Duplicate the pool to test the picker at the size that motivated it.
 *
 * The searchable modal exists because a Base with the full mod set behind it has several hundred groups
 * rather than 66, and that claim is worth testing rather than asserting. ×5 is 330 groups — the number
 * the owner named. Copies are labelled so nothing synthetic can be mistaken for real tier data.
 */
const inflate = (base: ModPool, factor: number): ModPool =>
  factor <= 1
    ? base
    : {
        ...base,
        groups: Array.from({ length: factor }).flatMap((_, copy) =>
          copy === 0
            ? base.groups
            : base.groups.map((group) => ({
                ...group,
                id: `${group.id}__stress${copy}`,
                lines: group.lines.map((line) => `[stress ${copy + 1}] ${line}`),
              })),
        ),
      };

export default function Shell() {
  const [variant, setVariant] = useState<VariantKey>(readVariant);
  const [knobs, setKnobs] = useState<Knobs>(defaultKnobs);
  const [driverOpen, setDriverOpen] = useState(true);
  const [ilvl, setIlvl] = useState(83);
  const [stress, setStress] = useState(1);
  const realPool = usePool();
  const pool = useMemo(() => inflate(realPool, stress), [realPool, stress]);

  const status = useMemo(() => mockStatus(knobs), [knobs]);
  const logLines = useMemo(() => mockLog(knobs), [knobs]);
  const group = useMemo(
    () => pool.groups.find((g) => g.id === status.targetGroup) ?? null,
    [pool, status.targetGroup],
  );

  const props: VariantProps = {
    status,
    pool,
    group,
    ilvl,
    setIlvl,
    lastRead:
      status.state === "Idle" || status.state === "Sighting"
        ? null
        : captures[knobs.captureIndex]?.text ?? null,
    logLines,
    // Stubs, per `UI.md`: a prototype answers "what should this look like", not "does the backend
    // work". These mutate the mock, which is also the only thing that *can* be mutated on a Mac.
    setTarget: (groupId, tierThreshold) => {
      const next = pool.groups.find((g) => g.id === groupId);
      const clamped = next
        ? Math.min(tierThreshold, Math.max(...next.tiers.map((t) => t.tier)))
        : 1;
      setKnobs({ ...knobs, targetGroup: groupId, tierThreshold: clamped });
    },
    setArmed: (on) =>
      setKnobs({
        ...knobs,
        state: on ? "Sighting" : "Idle",
        rolls: on ? 0 : knobs.rolls,
        lastTier: null,
        consecutiveUnknown: 0,
      }),
    acknowledge: () =>
      setKnobs({ ...knobs, state: "Sighting", rolls: 0, lastTier: null, consecutiveUnknown: 0 }),
  };

  return (
    <>
      {variant === "live" ? (
        // Today's window, unmodified, as the control. This is the one variant that needs the real
        // backend — under `pnpm dev` its `invoke()` calls fail and it shows errors, which is correct.
        <App />
      ) : (
        <main className={driverOpen ? "pv-main pv-main-shifted" : "pv-main"}>
          {variant === "A" && <VariantA {...props} />}
          {variant === "B" && <VariantB {...props} />}
          {variant === "C" && <VariantC {...props} />}
          {variant === "D" && <VariantD {...props} />}
        </main>
      )}

      {variant !== "live" && (
        <Driver
          knobs={knobs}
          setKnobs={setKnobs}
          open={driverOpen}
          setOpen={setDriverOpen}
          stress={stress}
          setStress={setStress}
          poolSize={pool.groups.length}
        />
      )}
      <Switcher current={variant} labels={labels} onChange={setVariant} />
    </>
  );
}

export { variantKeys };
