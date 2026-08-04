/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * The floating variant bar, per `UI.md`: left arrow, label, right arrow, plus `←`/`→`.
 *
 * The wrinkle this project has: the app normally runs in a **Tauri webview with no URL bar**, so
 * `?variant=` cannot be typed. The arrows and the keys are therefore the real mechanism and the search
 * param is only a record of where you are, written with `history.replaceState` so a reload keeps the
 * variant instead of snapping back to the first one.
 *
 * That said — `src/prototype/data.ts` falls back to reading the tier data straight off disk when
 * `invoke()` is unavailable, which means `pnpm dev` in an ordinary browser works and gives back the URL
 * bar, devtools and a reload that is not a whole app restart. That is the better loop; `pnpm tauri dev`
 * is only needed to judge variant `live`, which does need the real backend.
 */
import { useEffect } from "react";

export const variantKeys = ["live", "A", "B", "C", "D"] as const;
export type VariantKey = (typeof variantKeys)[number];

export const readVariant = (): VariantKey => {
  const found = new URLSearchParams(window.location.search).get("variant");
  return (variantKeys as readonly string[]).includes(found ?? "")
    ? (found as VariantKey)
    : "live";
};

const writeVariant = (key: VariantKey) => {
  const url = new URL(window.location.href);
  url.searchParams.set("variant", key);
  window.history.replaceState(null, "", url);
};

/** True when a keystroke belongs to something the human is typing into. */
const typing = () => {
  const active = document.activeElement;
  if (!active) return false;
  const tag = active.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    (active as HTMLElement).isContentEditable
  );
};

export function Switcher({
  current,
  labels,
  onChange,
}: {
  current: VariantKey;
  labels: Record<VariantKey, string>;
  onChange: (key: VariantKey) => void;
}) {
  const step = (delta: number) => {
    const at = variantKeys.indexOf(current);
    const next = variantKeys[(at + delta + variantKeys.length) % variantKeys.length];
    writeVariant(next);
    onChange(next);
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (typing() || event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        step(-1);
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        step(1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <div className="pb-bar">
      <button onClick={() => step(-1)} aria-label="Previous variant">
        ←
      </button>
      <span className="pb-bar-label">
        <strong>{current === "live" ? "LIVE" : current}</strong> {labels[current]}
      </span>
      <button onClick={() => step(1)} aria-label="Next variant">
        →
      </button>
      <span className="pb-bar-keys">← →</span>
    </div>
  );
}
