/**
 * Choosing a Mod Group: a searchable modal.
 *
 * A modal rather than a `<select>` because of where this is going. This Base has 66 groups, which is
 * already a scroll; a Base with the game's full mod set behind it is several hundred, and a dropdown with
 * several hundred options is not a choice. A modal has room for a search field, grouped sections, and
 * enough per row to tell two similar mods apart.
 *
 * ## The one rule this component exists to enforce
 *
 * **A group is chosen. An affix name is never chosen.** A group's display name is per tier — the group
 * behind *Minions deal # to # additional Physical Damage* is `Flaring` at T1 and `Annealed` at T4 — so a
 * list offering `Flaring` and `Annealed` as separate entries is offering the same mod twice and lying
 * about both. That is the trap [#4](https://github.com/Furizaa/poe-graft/issues/4) walked into.
 *
 * Searching by name is still worth having, because that is how mods are talked about. So a name may
 * **match** a row but never **label** one: the label is always the rendered line, and a name match is
 * reported as *matches "Annealed" at T4*. Both `flaring` and `annealed` find the one group.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import type { ModGroup, ModPool } from "./api";

/** `Minions deal # to # additional Physical Damage` — the label, never an affix name. */
const label = (group: ModGroup) => group.lines.join(" · ");

/** `23–26 to 33–39`, the numbers the game will print. */
const bands = (group: ModGroup, tier: number) => {
  const found = group.tiers.find((t) => t.tier === tier);
  if (!found) return "";
  return found.bands.map(([min, max]) => (min === max ? `${min}` : `${min}–${max}`)).join(" to ");
};

const escapeRegExp = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** Wrap matched tokens, so a long line shows at a glance why it is in the list. */
function Highlighted({ text, tokens }: { text: string; tokens: string[] }) {
  if (tokens.length === 0) return <>{text}</>;
  const pattern = new RegExp(`(${tokens.map(escapeRegExp).join("|")})`, "gi");
  const lowered = tokens.map((token) => token.toLowerCase());
  return (
    <>
      {text
        .split(pattern)
        .filter((part) => part !== "")
        .map((part, index) =>
          lowered.includes(part.toLowerCase()) ? (
            <mark key={index}>{part}</mark>
          ) : (
            <span key={index}>{part}</span>
          ),
        )}
    </>
  );
}

/** A group that survived the search, and which tier's name got it there. */
type Match = {
  group: ModGroup;
  viaName: { name: string; tier: number } | null;
};

/**
 * Every token has to be accounted for, by the rendered line or by some tier's affix name.
 *
 * AND rather than OR, and order-independent, so `minion phys` narrows to one group instead of returning
 * everything mentioning minions.
 */
function match(group: ModGroup, tokens: string[]): Match | null {
  const line = label(group).toLowerCase();
  const unmatched = tokens.filter((token) => !line.includes(token));
  if (unmatched.length === 0) return { group, viaName: null };

  let viaName: Match["viaName"] = null;
  for (const token of unmatched) {
    const found = group.tiers.find((tier) => tier.affixName.toLowerCase().includes(token));
    if (!found) return null;
    viaName ??= { name: found.affixName, tier: found.tier };
  }
  return { group, viaName };
}

export default function ModPicker({
  pool,
  current,
  onPick,
  onClose,
}: {
  pool: ModPool;
  /** The group id currently targeted, so it can be marked in the list. */
  current: string;
  onPick: (groupId: string) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  const tokens = useMemo(() => query.toLowerCase().split(/\s+/).filter(Boolean), [query]);

  const { prefixes, suffixes, flat } = useMemo(() => {
    const found = pool.groups
      .map((group) => match(group, tokens))
      .filter((hit): hit is Match => hit !== null);
    const prefixes = found.filter((hit) => hit.group.generation === "prefix");
    const suffixes = found.filter((hit) => hit.group.generation === "suffix");
    return { prefixes, suffixes, flat: [...prefixes, ...suffixes] };
  }, [pool, tokens]);

  // A new result set invalidates the cursor's position, so it goes back to the top.
  useEffect(() => setActive(0), [query]);

  useEffect(() => {
    listRef.current?.querySelector('[data-active="true"]')?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const choose = (groupId: string) => {
    onPick(groupId);
    onClose();
  };

  // Handled on the dialog rather than the window, so nothing behind the modal sees these keys.
  const onKeyDown = (event: React.KeyboardEvent) => {
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
      if (hit) choose(hit.group.id);
    }
  };

  const row = (hit: Match, index: number) => {
    const best = hit.group.tiers.reduce((a, b) => (a.tier <= b.tier ? a : b));
    const isActive = index === active;
    const tierCount = hit.group.tiers.length;
    return (
      <button
        key={hit.group.id}
        type="button"
        data-active={isActive}
        className={[
          "pick-row",
          isActive ? "pick-row-active" : "",
          hit.group.id === current ? "pick-row-current" : "",
        ]
          .filter(Boolean)
          .join(" ")}
        onMouseEnter={() => setActive(index)}
        onClick={() => choose(hit.group.id)}
      >
        <span className="pick-row-line">
          <Highlighted text={label(hit.group)} tokens={tokens} />
        </span>
        <span className="pick-row-meta">
          <span className="badge">
            {tierCount} {tierCount === 1 ? "tier" : "tiers"}
          </span>
          <span className="badge">
            T{best.tier} {bands(hit.group, best.tier)}
          </span>
          <span className="badge">ilvl {best.requiredIlvl}</span>
          {hit.viaName && (
            <span className="badge good">
              matches “{hit.viaName.name}” at T{hit.viaName.tier}
            </span>
          )}
        </span>
      </button>
    );
  };

  return (
    <div className="pick-backdrop" onMouseDown={onClose}>
      <div
        className="pick-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Choose a Mod Group"
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="pick-head">
          <input
            ref={searchRef}
            className="grow"
            value={query}
            placeholder="Search — “minion phys”, “life”, or a name like “annealed”"
            onChange={(event) => setQuery(event.target.value)}
          />
          <span className="muted small mono">
            {flat.length}/{pool.groups.length}
          </span>
          <button type="button" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>

        <div className="pick-list" ref={listRef}>
          {flat.length === 0 && (
            <p className="muted small pick-empty">
              Nothing matches. Names are per tier, so “Flaring” and “Annealed” both find the same group.
            </p>
          )}

          {prefixes.length > 0 && (
            <>
              <div className="pick-section">Prefixes · {prefixes.length}</div>
              {prefixes.map((hit, index) => row(hit, index))}
            </>
          )}

          {suffixes.length > 0 && (
            <>
              <div className="pick-section">Suffixes · {suffixes.length}</div>
              {suffixes.map((hit, index) => row(hit, prefixes.length + index))}
            </>
          )}
        </div>

        <div className="pick-foot">
          <span className="muted small">
            <span className="mono">↑↓</span> move · <span className="mono">↵</span> choose ·{" "}
            <span className="mono">esc</span> close
          </span>
          <span className="muted small">A group is chosen, never an affix name.</span>
        </div>
      </div>
    </div>
  );
}
