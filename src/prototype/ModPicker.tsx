/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * The searchable Mod Group modal — variant A's picker, replacing the `<select>`.
 *
 * The reason it is a modal and not a dropdown is scale rather than taste. This Base has 66 groups; a
 * Base with the full mod set behind it has several hundred, and a `<select>` with 300 options is a
 * scroll, not a choice. A modal buys three things a dropdown cannot have: a search field, grouped
 * sections with headers, and room for each row to carry the numbers that make it identifiable.
 *
 * ## What is searched, and the trap it has to avoid
 *
 * Tokens are matched (AND, in any order) against **the group's rendered lines** and **every tier's affix
 * name** — so `phys minion`, `annealed` and `flaring` all find the same group. Searching affix names is
 * worth having because that is how mods are talked about, but it is exactly the trap
 * [#4](https://github.com/Furizaa/poe-graft/issues/4) fell into: a group's name is **per tier**, so
 * `Flaring` and `Annealed` are one group, not two. The resolution is that an affix name may *match* a row
 * but never *label* one — the label is always the rendered line, and a name match is shown as a hint
 * chip saying which tier it came from. Nothing in this list can be picked by a name.
 *
 * ## Unreachable mods are listed, and disabled
 *
 * A mod an Orb of Alteration cannot roll is a craft that can never hit. If those mods were simply absent,
 * a search for one would look like a broken search. They are shown in their own greyed section instead,
 * so the absence explains itself. See `unreachableMods` in `data.ts`.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import type { ModGroup, ModPool } from "../api";
import { bands, label, unreachableMods } from "./data";

const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** Wrap every matched token so a long line shows why it is in the list. */
const highlight = (text: string, tokens: string[]) => {
  if (tokens.length === 0) return text;
  const pattern = new RegExp(`(${tokens.map(escape).join("|")})`, "gi");
  const lowered = tokens.map((t) => t.toLowerCase());
  return text
    .split(pattern)
    .filter((part) => part !== "")
    .map((part, index) =>
      lowered.includes(part.toLowerCase()) ? (
        <mark key={index} className="pk-mark">
          {part}
        </mark>
      ) : (
        <span key={index}>{part}</span>
      ),
    );
};

type Hit = {
  group: ModGroup;
  /** Which tier's affix name matched, when the match came from a name rather than the line. */
  viaName: { name: string; tier: number } | null;
};

const matches = (group: ModGroup, tokens: string[]): Hit | null => {
  const line = label(group).toLowerCase();
  const names = group.tiers.map((t) => ({ name: t.affixName, tier: t.tier }));

  const unmatched = tokens.filter((token) => !line.includes(token));
  if (unmatched.length === 0) return { group, viaName: null };

  // Every token the line did not account for has to be covered by an affix name.
  let viaName: { name: string; tier: number } | null = null;
  for (const token of unmatched) {
    const found = names.find((n) => n.name.toLowerCase().includes(token));
    if (!found) return null;
    viaName ??= found;
  }
  return { group, viaName };
};

export function ModPicker({
  pool,
  current,
  onPick,
  onClose,
}: {
  pool: ModPool;
  current: string;
  onPick: (groupId: string) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const tokens = useMemo(
    () => query.toLowerCase().split(/\s+/).filter(Boolean),
    [query],
  );

  const { prefixes, suffixes, flat, blocked } = useMemo(() => {
    const hits = pool.groups
      .map((group) => matches(group, tokens))
      .filter((hit): hit is Hit => hit !== null);
    const prefixes = hits.filter((h) => h.group.generation === "prefix");
    const suffixes = hits.filter((h) => h.group.generation === "suffix");
    const blocked = tokens.length
      ? unreachableMods.filter((mod) => {
          const line = mod.line.toLowerCase();
          return tokens.every((token) => line.includes(token));
        })
      : [];
    return { prefixes, suffixes, flat: [...prefixes, ...suffixes], blocked };
  }, [pool, tokens]);

  // Reset the cursor whenever the result set changes, or it points at a row that has gone.
  useEffect(() => {
    setActive(0);
  }, [query]);

  useEffect(() => {
    listRef.current
      ?.querySelector('[data-active="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const commit = (groupId: string) => {
    onPick(groupId);
    onClose();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    // Stopped here so the prototype switcher's ←/→ and the page behind never see modal keys.
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((at) => Math.min(at + 1, flat.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((at) => Math.max(at - 1, 0));
    } else if (event.key === "Enter") {
      event.preventDefault();
      const hit = flat[active];
      if (hit) commit(hit.group.id);
    }
  };

  const row = (hit: Hit, index: number) => {
    const best = hit.group.tiers.reduce((a, b) => (a.tier <= b.tier ? a : b));
    const isActive = index === active;
    return (
      <button
        key={hit.group.id}
        data-active={isActive}
        className={`pk-row${isActive ? " pk-row-active" : ""}${hit.group.id === current ? " pk-row-current" : ""}`}
        onMouseEnter={() => setActive(index)}
        onClick={() => commit(hit.group.id)}
      >
        <span className="pk-row-line">{highlight(label(hit.group), tokens)}</span>
        <span className="pk-row-meta">
          <span className="pk-chip">
            {hit.group.tiers.length} {hit.group.tiers.length === 1 ? "tier" : "tiers"}
          </span>
          <span className="pk-chip">
            T{best.tier} {bands(hit.group, best.tier)}
          </span>
          <span className="pk-chip">ilvl {best.requiredIlvl}</span>
          {hit.viaName && (
            <span className="pk-chip pk-chip-name">
              matches “{hit.viaName.name}” at T{hit.viaName.tier}
            </span>
          )}
        </span>
      </button>
    );
  };

  return (
    <div className="pk-backdrop" onMouseDown={onClose}>
      <div
        className="pk-modal"
        role="dialog"
        aria-label="Choose a mod group"
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="pk-head">
          <input
            ref={inputRef}
            className="pk-search"
            value={query}
            placeholder="Search — try “minion phys”, “life”, or an affix name like “annealed”"
            onChange={(event) => setQuery(event.target.value)}
          />
          <span className="pk-count">
            {flat.length} of {pool.groups.length}
          </span>
          <button className="pk-close" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>

        <div className="pk-list" ref={listRef}>
          {flat.length === 0 && blocked.length === 0 && (
            <p className="pk-empty">
              Nothing matches. Names are per tier — searching “Annealed” and “Flaring” both find the same
              group.
            </p>
          )}

          {prefixes.length > 0 && (
            <>
              <div className="pk-section">Prefixes · {prefixes.length}</div>
              {prefixes.map((hit, index) => row(hit, index))}
            </>
          )}

          {suffixes.length > 0 && (
            <>
              <div className="pk-section">Suffixes · {suffixes.length}</div>
              {suffixes.map((hit, index) => row(hit, prefixes.length + index))}
            </>
          )}

          {/* Present so a search that finds nothing selectable still explains itself. */}
          {blocked.length > 0 && (
            <>
              <div className="pk-section pk-section-off">
                An Orb of Alteration cannot roll these · {blocked.length}
              </div>
              {blocked.map((mod) => (
                <div key={mod.id} className="pk-row pk-row-off">
                  <span className="pk-row-line">{highlight(mod.line, tokens)}</span>
                  <span className="pk-row-meta">
                    <span className="pk-chip">{mod.category}</span>
                  </span>
                </div>
              ))}
            </>
          )}
        </div>

        <div className="pk-foot">
          <span>
            <kbd>↑</kbd>
            <kbd>↓</kbd> move · <kbd>↵</kbd> choose · <kbd>esc</kbd> close
          </span>
          <span className="pk-foot-note">
            A group is chosen, never an affix name — the name changes per tier.
          </span>
        </div>
      </div>
    </div>
  );
}
