"""ONE-OFF: the tier-range overlap census for #23. Not part of the build.

Answers, from the RePoE export: within one Mod Group, on every non-unique flask Base
and every non-unique Base in the five armour slots, do the *display* bands of two
tiers ever overlap — and do Essence-only mods land in a normal tier's band?

It reuses scripts/build-mod-pool.py's derivation verbatim where it can:
`match_lines()`, `_pick_english()`, `handler_of()` and the tier-numbering rule are
copies, so the census compares the same numbers the generator would write. The one
deliberate difference is `apply_handlers()`, whose table is extended — the generator
raises `SystemExit` on an index_handler it does not know, and this scope reaches many
more of them. Handlers still unknown are counted and reported rather than skipped
silently, and every derived band is checked against RePoE's own rendered `text`
exactly as generator self-check 2 does.

Usage (upstream is the same ~75 MB the generator's docstring fetches, plus essences.json):

    UPSTREAM=.upstream OUT=/tmp/census python3 scripts/census-tier-overlaps.py

Writes OUT.json (the full pair list, for re-reading) and prints the report tables.
"""
import json, os, itertools, re, collections, random

DL = os.environ.get('UPSTREAM', '.upstream')
OUT = os.environ.get('OUT', '/tmp/census-tier-overlaps')

VERSION = open(os.path.join(DL, 'version.txt')).read().strip()
mods = json.load(open(os.path.join(DL, 'mods.json')))
mods_by_base = json.load(open(os.path.join(DL, 'mods_by_base.json')))
base_items = json.load(open(os.path.join(DL, 'base_items.json')))
stat_translations = json.load(open(os.path.join(DL, 'stat_translations.json')))
essences = json.load(open(os.path.join(DL, 'essences.json')))

# In scope: the map's two crafts. mods_by_base display name -> is this an armour slot?
SCOPE = {
    'Life Flasks': False, 'Mana Flasks': False, 'Hybrid Flasks': False,
    'Utility Flasks': False,
    'Body Armours': True, 'Boots': True, 'Gloves': True, 'Helmets': True,
    'Shields': True,
}

# ------------------------------------------------- copied from build-mod-pool.py
st_by_ids = {tuple(t['ids']): t for t in stat_translations}


def _cond_ok(cond, v):
    if cond.get('min') is not None and v < cond['min']:
        ok = False
    elif cond.get('max') is not None and v > cond['max']:
        ok = False
    else:
        ok = True
    return (not ok) if cond.get('negated') else ok


def _pick_english(t, ranges):
    both = either = None
    for en in t['English']:
        conds = en['condition']
        full = all(_cond_ok(c, lo) and _cond_ok(c, hi)
                   for c, (lo, hi) in zip(conds, ranges))
        part = all(_cond_ok(c, lo) or _cond_ok(c, hi)
                   for c, (lo, hi) in zip(conds, ranges))
        if full and both is None:
            both = en
        if part and either is None:
            either = en
    return both or either or t['English'][0]


def _placeholderise(t, ranges):
    en = _pick_english(t, ranges)
    s = en['string']
    for i in range(len(en['format'])):
        s = s.replace('{%d}' % i, '#').replace('{%d:+d}' % i, '#')
    trade = [x['id'] for x in (t['trade_stats'] or []) if x['type'] == 'explicit']
    handlers = sorted({h for hs in en['index_handlers'] for h in hs})
    return s, (trade[0] if trade else None), handlers


def match_lines(mod):
    by_id = {s['id']: (s['min'], s['max']) for s in mod['stats']}
    remaining = [s['id'] for s in mod['stats'] if not (s['min'] == 0 and s['max'] == 0)]
    lines = []
    while remaining:
        hit = None
        for n in range(len(remaining), 0, -1):
            for combo in itertools.combinations(remaining, n):
                if combo in st_by_ids:
                    hit = combo
                    break
            if hit:
                break
        if not hit:
            lines.append({'match_string': None, 'stat_ids': list(remaining),
                          'index_handlers': []})
            break
        s, _trade, handlers = _placeholderise(st_by_ids[hit], [by_id[i] for i in hit])
        lines.append({'match_string': s, 'stat_ids': list(hit),
                      'index_handlers': handlers})
        remaining = [i for i in remaining if i not in hit]
    return lines


def handler_of(lines, stat_id):
    for l in lines:
        if stat_id in l['stat_ids']:
            return l['index_handlers']
    return []


# ------------------------------------ apply_handlers, extended past the generator
UNHANDLED = collections.Counter()
_IDENT = {'mod_value_to_item_class', 'canonical_line', 'display_indexable_support',
          'tree_expansion_jewel_passive', 'passive_hash', 'affliction_x',
          'affliction_reverse_reservation', 'reminderstring',
          'affliction_display_reservation_as_flat_reservation',
          'metamorphosis_reward_description', 'display_essence_monster_type'}
_SCALE = {
    'divide_by_two_0dp': (2, 0), 'divide_by_three': (3, None),
    'divide_by_four': (4, None), 'divide_by_five': (5, None),
    'divide_by_six': (6, None), 'divide_by_ten_0dp': (10, 0),
    'divide_by_ten_1dp': (10, 1), 'divide_by_ten_1dp_if_required': (10, 1),
    'divide_by_twelve': (12, None), 'divide_by_fifteen_0dp': (15, 0),
    'divide_by_twenty': (20, None), 'divide_by_fifty': (50, None),
    'divide_by_one_hundred': (100, None), 'divide_by_one_hundred_2dp': (100, 2),
    'divide_by_one_hundred_2dp_if_required': (100, 2),
    'divide_by_one_thousand': (1000, None),
    'milliseconds_to_seconds': (1000, None),
    'milliseconds_to_seconds_0dp': (1000, 0),
    'milliseconds_to_seconds_1dp': (1000, 1),
    'milliseconds_to_seconds_2dp': (1000, 2),
    'milliseconds_to_seconds_2dp_if_required': (1000, 2),
    'per_minute_to_per_second': (60, 1), 'per_minute_to_per_second_0dp': (60, 0),
    'per_minute_to_per_second_1dp': (60, 1), 'per_minute_to_per_second_2dp': (60, 2),
    'per_minute_to_per_second_2dp_if_required': (60, 2),
}


def apply_handlers(v, handlers):
    """Raw stat value -> the value as rendered in the tooltip. None = unknown handler."""
    for h in handlers:
        if h in _IDENT:
            continue
        if h in _SCALE:
            d, dp = _SCALE[h]
            v = v // d if dp == 0 else (v / d if dp is None else round(v / d, dp))
        elif h == 'negate':
            v = -v
        elif h == 'divide_by_one_hundred_and_negate':
            v = -v / 100
        elif h == 'divide_by_one_hundred_and_negate_2dp':
            v = round(-v / 100, 2)
        elif h == 'negate_and_double':
            v = -v * 2
        elif h == 'multiplicative_damage_modifier':
            v = v + 100
        elif h == 'times_twenty':
            v = v * 20
        elif h == 'times_one_point_five':
            v = v * 1.5
        elif h == 'double':
            v = v * 2
        elif h == 'thirty_percent_of_value':
            v = v * 0.3
        elif h == 'old_leech_percent':
            v = v / 100
        elif h == 'old_leech_permyriad':
            v = v / 10000
        elif h == 'divide_by_twenty_then_double_0dp':
            v = (v / 20) * 2 // 1
        else:
            UNHANDLED[h] += 1
            return None
    return int(v) if float(v).is_integer() else v


# ---------------------------------------------------------------- tier records
def tier_of(mid, weight, source):
    m = mods[mid]
    lines = match_lines(m)
    stats = []
    for s in m['stats']:
        hs = handler_of(lines, s['id'])
        stats.append({'id': s['id'], 'min': s['min'], 'max': s['max'],
                      'display_min': apply_handlers(s['min'], hs),
                      'display_max': apply_handlers(s['max'], hs)})
    return {
        'mod_id': mid, 'affix_name': m['name'], 'required_ilvl': m['required_level'],
        'match_strings': [l['match_string'] for l in lines],
        # What the matcher actually keys on: the rendered lines *and* the stat ids
        # behind them, in render order. Two tiers are only confusable if these agree.
        'signature': [[l['match_string'], l['stat_ids']] for l in lines],
        'n_bands': len(stats),
        'stats': stats, 'text': m['text'], 'spawn_weight': weight,
        'source': source, 'gen': m['generation_type'],
        'royale': 'Royale' in mid,
        'essence_only': bool(m.get('is_essence_only')),
    }


ESS_KEY = {'Body Armours': 'Body Armour', 'Boots': 'Boots', 'Gloves': 'Gloves',
           'Helmets': 'Helmet', 'Shields': 'Shield'}
ESS_META = {}          # mod_id -> [(essence display name, is_corruption_only)]
for eid, e in essences.items():
    for ck, mid in (e['mods'] or {}).items():
        ESS_META.setdefault(mid, []).append((e['name'], e['type']['is_corruption_only']))


def base_groups():
    out = []
    for cls in SCOPE:
        for tagkey, entry in mods_by_base[cls].items():
            if tagkey == 'essences' or 'bases' not in entry:
                continue
            names, states = [], collections.Counter()
            for b in entry['bases']:
                bb = base_items.get(b)
                states[bb['release_state'] if bb else 'MISSING'] += 1
                if bb:
                    names.append(bb['name'])
            if not states['released']:
                out.append({'item_class': cls, 'tags': tagkey, 'skipped': True,
                            'base_names': sorted(names), 'states': dict(states)})
                continue
            cands = []
            for gen in ('prefix', 'suffix'):
                for group, ids in entry['mods'].get(gen, {}).items():
                    for mid, w in ids.items():
                        cands.append((group, tier_of(mid, w, 'pool')))
            ess = mods_by_base[cls].get('essences', {}) if cls in ESS_KEY else {}
            for _ename, byrank in ess.items():
                for _rank, mid in byrank.items():
                    m = mods[mid]
                    g = m['groups'][0] if m['groups'] else '(no group)'
                    if any(c[0] == g and c[1]['mod_id'] == mid for c in cands):
                        continue          # essence grants an ordinary pool mod
                    if any(c[1]['mod_id'] == mid for c in cands):
                        continue
                    t = tier_of(mid, 0, 'essence')
                    t['essences'] = ESS_META.get(mid, [])
                    cands.append((g, t))
            out.append({'item_class': cls, 'tags': tagkey, 'skipped': False,
                        'base_names': sorted(names), 'states': dict(states),
                        'n_bases': len(entry['bases']), 'candidates': cands})
    return out


def ladder(cands, keep):
    """group -> tiers, numbered GGG-style: highest required_ilvl in the group = tier 1."""
    groups = collections.defaultdict(list)
    for g, t in cands:
        if keep(t):
            groups[g].append(dict(t))
    for g, ts in groups.items():
        ts.sort(key=lambda r: (-r['required_ilvl'], r['mod_id']))
        for i, t in enumerate(ts, start=1):
            t['tier'] = i
    return groups


def band(s):
    """Normalised display band. `negate` inverts the bounds, so min>max happens."""
    lo, hi = s['display_min'], s['display_max']
    if lo is None or hi is None:
        return None
    return (lo, hi) if lo <= hi else (hi, lo)


def confusable(a, b):
    """Could one rendered mod line be read as either tier?

    Requires (1) the same rendered lines over the same stat ids in the same order —
    otherwise the two tiers are different *text* and the matcher never compares them —
    and (2) every band overlapping, which is `build-mod-pool.py` self-check 3's test.
    """
    if a['signature'] != b['signature']:
        return False
    if len(a['stats']) != len(b['stats']):
        return False
    for x, y in zip(a['stats'], b['stats']):
        bx, by = band(x), band(y)
        if bx is None or by is None:
            return False
        if not (bx[0] <= by[1] and by[0] <= bx[1]):
            return False
    return True


def overlaps(a, b):
    """Exactly `build-mod-pool.py` self-check 3: bands only, no text check."""
    if len(a['stats']) != len(b['stats']):
        return False
    for x, y in zip(a['stats'], b['stats']):
        bx, by = band(x), band(y)
        if bx is None or by is None:
            return False
        if not (bx[0] <= by[1] and by[0] <= bx[1]):
            return False
    return True


def identical(a, b):
    return (len(a['stats']) == len(b['stats'])
            and all(band(x) == band(y) for x, y in zip(a['stats'], b['stats'])))


def ambiguous_share(t, others, cap=200000):
    """P(a uniformly random roll of tier `t` is also accepted by some other tier)."""
    spans = [(s['min'], s['max']) for s in t['stats']]
    total = 1
    for lo, hi in spans:
        total *= (hi - lo + 1)
    def hit(vec):
        for o in others:
            if len(o['stats']) != len(t['stats']):
                continue
            if all(band(os_)[0] <= v <= band(os_)[1]
                   for v, os_ in zip(vec, o['stats']) if band(os_)):
                return True
        return False
    # compare in display units: transform each raw value with this tier's handlers
    def disp(vec):
        out = []
        for v, s in zip(vec, t['stats']):
            lo, hi = s['min'], s['max']
            b = band(s)
            if b is None or hi == lo:
                out.append(b[0] if b else v)
            else:
                out.append(b[0] + (v - lo) * (b[1] - b[0]) / (hi - lo))
        return out
    if total <= cap:
        n = sum(1 for vec in itertools.product(*[range(lo, hi + 1) for lo, hi in spans])
                if hit(disp(vec)))
        return n / total, total, True
    rng = random.Random(1234)
    N = 20000
    n = sum(1 for _ in range(N)
            if hit(disp([rng.randint(lo, hi) for lo, hi in spans])))
    return n / N, total, False


# ---------------------------------------------------------------- run
BG = base_groups()
live = [b for b in BG if not b['skipped']]
VARIANTS = {
    'as_generated': lambda t: t['source'] == 'pool',
    'royale_dropped': lambda t: t['source'] == 'pool' and not t['royale'],
    'royale_dropped_plus_essence': lambda t: not t['royale'],
}

report = {'game_version': VERSION, 'variants': {},
          'skipped_base_groups': [{k: v for k, v in b.items() if k != 'candidates'}
                                  for b in BG if b['skipped']],
          'base_group_index': [{'item_class': b['item_class'], 'tags': b['tags'],
                                'n_bases': b['n_bases'], 'base_names': b['base_names'],
                                'states': b['states']} for b in live]}

for vname, keep in VARIANTS.items():
    rows = []
    for b in live:
        gs = ladder(b['candidates'], keep)
        pairs, collisions, unresolved, inverted = [], [], [], []
        split_groups, band_count_split = [], []
        for gname, ts in sorted(gs.items()):
            sigs = {json.dumps(t['signature']) for t in ts}
            if len(sigs) > 1:
                # build-mod-pool.py asserts every tier of a group renders identically
                split_groups.append({'group': gname, 'n_signatures': len(sigs),
                                     'n_tiers': len(ts),
                                     'texts': sorted({t['match_strings'][0] or '?'
                                                      for t in ts})[:6]})
            if len({t['n_bands'] for t in ts}) > 1:
                band_count_split.append(gname)
            for t in ts:
                for s in t['stats']:
                    if (s['display_min'] is not None
                            and s['display_min'] > s['display_max']):
                        inverted.append((gname, t['mod_id'], s['id']))
            for i, a in enumerate(ts):
                for c in ts[i + 1:]:
                    if not overlaps(a, c):
                        continue
                    pairs.append({
                        'group': gname, 'gen': a['gen'],
                        'confusable': confusable(a, c),
                        'a': a['mod_id'], 'b': c['mod_id'],
                        'a_tier': a['tier'], 'b_tier': c['tier'],
                        'a_ilvl': a['required_ilvl'], 'b_ilvl': c['required_ilvl'],
                        'a_name': a['affix_name'], 'b_name': c['affix_name'],
                        'a_text': a['text'], 'b_text': c['text'],
                        'a_src': a['source'], 'b_src': c['source'],
                        'a_royale': a['royale'], 'b_royale': c['royale'],
                        'a_ess_only': a['essence_only'], 'b_ess_only': c['essence_only'],
                        'identical': identical(a, c),
                        'same_ilvl': a['required_ilvl'] == c['required_ilvl'],
                    })
        seen = {}
        for gname, ts in sorted(gs.items()):
            if not ts:
                continue
            for ms in ts[0]['match_strings']:
                if ms is None:
                    unresolved.append(gname)
                elif ms in seen and seen[ms] != gname:
                    collisions.append({'match_string': ms,
                                       'groups': sorted([seen[ms], gname])})
                else:
                    seen[ms] = gname
        rows.append({'item_class': b['item_class'], 'tags': b['tags'],
                     'n_groups': len(gs), 'n_tiers': sum(len(t) for t in gs.values()),
                     'affected_groups': sorted({p['group'] for p in pairs
                                                if p['confusable']}),
                     'pairs': pairs, 'collisions': collisions,
                     'unresolved': unresolved, 'inverted_bands': inverted,
                     'split_groups': split_groups,
                     'band_count_split': band_count_split,
                     'would_generate': not (pairs or collisions or unresolved
                                            or split_groups)})
    report['variants'][vname] = rows

# --------------------------------- does folding essence mods in renumber the ladder?
# The annotation cross-check compares the game's `(Tier: N)` against our derived N.
# If an essence mod joins the group's ladder, every normal tier below it shifts.
renum = {}
for b in live:
    plain = ladder(b['candidates'], VARIANTS['royale_dropped'])
    withe = ladder(b['candidates'], VARIANTS['royale_dropped_plus_essence'])
    for gname, ts in plain.items():
        key = (b['item_class'], gname)
        if key in renum:
            continue
        after = {t['mod_id']: t['tier'] for t in withe.get(gname, [])}
        moved = [t['mod_id'] for t in ts if after.get(t['mod_id']) != t['tier']]
        n_ess = sum(1 for t in withe.get(gname, []) if t['source'] == 'essence')
        if n_ess:
            renum[key] = {'item_class': b['item_class'], 'group': gname,
                          'pool_tiers': len(ts), 'essence_tiers': n_ess,
                          'renumbered': len(moved), 'moved': moved[:8]}
report['renumbering'] = {'%s|%s' % k: v for k, v in renum.items()}

# ------------------------------------------------- per-roll ambiguity, one variant
amb = {}
for b in live:
    gs = ladder(b['candidates'], VARIANTS['royale_dropped_plus_essence'])
    for gname, ts in gs.items():
        key = (b['item_class'], gname)
        if key in amb:
            continue
        tot_w = sum(t['spawn_weight'] for t in ts) or 1
        num = 0.0
        per = []
        for t in ts:
            others = [o for o in ts if o['mod_id'] != t['mod_id']
                      and o['signature'] == t['signature']
                      and len(o['stats']) == len(t['stats'])]
            share, space, exact = ambiguous_share(t, others)
            per.append({'mod_id': t['mod_id'], 'tier': t['tier'], 'share': share,
                        'space': space, 'exact': exact,
                        'weight': t['spawn_weight']})
            num += share * t['spawn_weight']
        if any(p['share'] > 0 for p in per):
            amb[key] = {'item_class': b['item_class'], 'group': gname,
                        'p_ambiguous_given_group': num / tot_w, 'tiers': per,
                        'tags': b['tags']}
report['ambiguity'] = {'%s|%s' % k: v for k, v in amb.items()}
report['unhandled_handlers'] = dict(UNHANDLED)

# ---------------------------------------------- self-check 2, on everything in scope
NUM = re.compile(r'-?\d+(?:\.\d+)?')
ok = bad = skip = 0
bad_ex = []
seen_mods = set()
for b in live:
    for _g, t in b['candidates']:
        if t['mod_id'] in seen_mods:
            continue
        seen_mods.add(t['mod_id'])
        got = [abs(float(x)) for x in NUM.findall(t['text'])]
        want, dead = [], False
        for s in t['stats']:
            if s['min'] == 0 and s['max'] == 0:
                continue
            if s['display_min'] is None:
                dead = True
                break
            lo, hi = float(s['display_min']), float(s['display_max'])
            want.extend([abs(lo)] if lo == hi else [abs(lo), abs(hi)])
        if dead:
            skip += 1
        elif got == want:
            ok += 1
        else:
            bad += 1
            if len(bad_ex) < 200:
                bad_ex.append({'mod_id': t['mod_id'], 'text': t['text'],
                               'derived': want, 'in_text': got})
report['text_selfcheck'] = {'ok': ok, 'bad': bad, 'unknown_handler': skip,
                            'examples': bad_ex}

json.dump(report, open(OUT + '.json', 'w'), indent=1)

# ---------------------------------------------------------------- print
print('RePoE version', VERSION)
print('in-scope Base Groups: %d live, %d skipped (no released base)'
      % (len(live), len(report['skipped_base_groups'])))
print('display-bound self-check: ok=%d bad=%d unknown-handler=%d' % (ok, bad, skip))
print('unhandled index_handlers:', dict(UNHANDLED) or 'none')
for vname, rows in report['variants'].items():
    print('\n== %s' % vname)
    conf = [p for r in rows for p in r['pairs'] if p['confusable']]
    dist = {(r['item_class'], p['group'], p['a'], p['b'])
            for r in rows for p in r['pairs'] if p['confusable']}
    print('   groups=%d tiers=%d band-overlap-pairs=%d confusable-pairs=%d '
          'distinct-confusable=%d affected-groups=%d'
          % (sum(r['n_groups'] for r in rows), sum(r['n_tiers'] for r in rows),
             sum(len(r['pairs']) for r in rows), len(conf), len(dist),
             len({(r['item_class'], g) for r in rows for g in r['affected_groups']})))
    print('   match_string collisions=%d (distinct %d)  unresolved match lines=%d  '
          'groups whose tiers render differently=%d (distinct %d)  inverted bands=%d'
          % (sum(len(r['collisions']) for r in rows),
             len({(c['match_string'], tuple(c['groups']))
                  for r in rows for c in r['collisions']}),
             sum(len(r['unresolved']) for r in rows),
             sum(len(r['split_groups']) for r in rows),
             len({(r['item_class'], s['group']) for r in rows for s in r['split_groups']}),
             sum(len(r['inverted_bands']) for r in rows)))
    print('   Base Groups the generator would refuse: %d of %d'
          % (sum(0 if r['would_generate'] else 1 for r in rows), len(rows)))
    print('   confusable & identical bands=%d  & same required_ilvl=%d'
          % (sum(1 for p in conf if p['identical']),
             sum(1 for p in conf if p['same_ilvl'])))
    by_group = collections.Counter((cls, g) for cls, g, _a, _b in dist)
    for (cls, g), n in by_group.most_common(15):
        print('     %-14s %-46s %d distinct pair(s)' % (cls, g, n))
print('\n== folding essence mods into the ladder renumbers normal tiers')
rn = list(renum.values())
print('   %d (class, group) pairs gain essence tiers; %d of them renumber at least one '
      'normal tier; %d normal tiers change number in total'
      % (len(rn), sum(1 for x in rn if x['renumbered']),
         sum(x['renumbered'] for x in rn)))
for x in sorted(rn, key=lambda x: -x['renumbered'])[:12]:
    print('     %-14s %-34s %d pool + %d essence tiers -> %d renumbered'
          % (x['item_class'], x['group'], x['pool_tiers'], x['essence_tiers'],
             x['renumbered']))

print('\n== per-Roll ambiguity, given the group rolled (essence mods folded in)')
rows = sorted(report['ambiguity'].values(),
              key=lambda x: -x['p_ambiguous_given_group'])
print('   %d (class, group) pairs have any ambiguous roll at all' % len(rows))
for x in rows[:20]:
    print('     %-14s %-34s P(ambiguous roll) = %.3f'
          % (x['item_class'], x['group'], x['p_ambiguous_given_group']))
print('\nwrote', OUT + '.json')
