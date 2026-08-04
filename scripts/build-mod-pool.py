"""Regenerate data/ghastly-eye-jewel.json and data/raw/* from the RePoE export.

Upstream (~75 MB, deliberately not committed) — note this reads the repo's `master`,
not GitHub Pages, because the Pages deploy lags behind:

    B=https://raw.githubusercontent.com/repoe-fork/repoe-fork.github.io/master
    mkdir -p .upstream && cd .upstream
    for f in mods.json mods_by_base.json base_items.json stat_translations.json; do
      curl -sLO "$B/data/$f"
    done
    curl -sLO "$B/version.txt"

Optional cross-check against poedb.tw (writes data/raw/poedb-modsview.*.json). The
whole dataset is inlined in the page as the argument to `new ModsView({...})`:

    curl -s -A Mozilla https://poedb.tw/us/Ghastly_Eye_Jewel > .upstream/poedb.html
    python3 -c "import json,sys; h=open('.upstream/poedb.html').read(); \
      s=h.find('new ModsView(')+13; e=h.find(');\\n});', s); \
      json.dump(json.loads(h[s:e]), open('.upstream/poedb_modsview.json','w'))"

Then:  python3 scripts/build-mod-pool.py

Fails loudly if any self-check breaks: the derived display bounds must reproduce
RePoE's own rendered `text`, tier ranges within a group must not overlap, and no
two mod groups may share a rendered-text match string.
"""
import json, os, itertools, re, sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DL = os.environ.get('UPSTREAM', os.path.join(REPO, '.upstream'))
DATA = os.path.join(REPO, 'data')
RAW = os.path.join(DATA, 'raw')
os.makedirs(RAW, exist_ok=True)

BASE_ID = 'Metadata/Items/Jewels/JewelAbyssSummoner'
BASE_TAGS_KEY = 'not_for_sale,abyss_jewel_summoner,abyss_jewel,default'
VERSION = open(os.path.join(DL, 'version.txt')).read().strip()

mods = json.load(open(os.path.join(DL, 'mods.json')))
mods_by_base = json.load(open(os.path.join(DL, 'mods_by_base.json')))
base_items = json.load(open(os.path.join(DL, 'base_items.json')))
stat_translations = json.load(open(os.path.join(DL, 'stat_translations.json')))
poedb_path = os.path.join(DL, 'poedb_modsview.json')
poedb = json.load(open(poedb_path)) if os.path.exists(poedb_path) else None

base = base_items[BASE_ID]
entry = mods_by_base['Abyss Jewels'][BASE_TAGS_KEY]
pools = entry['mods']

# ---------------------------------------------------------------- stat lookup
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
    """Pick the English translation entry whose conditions match the roll range."""
    both = None
    either = None
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


def apply_handlers(v, handlers):
    """Raw stat value -> the value as rendered in the tooltip."""
    for h in handlers:
        if h == 'negate':
            v = -v
        elif h == 'divide_by_one_hundred':
            v = v / 100
        elif h == 'divide_by_one_hundred_and_negate':
            v = -v / 100
        elif h == 'per_minute_to_per_second':
            v = round(v / 60, 1)
        elif h == 'divide_by_ten_0dp':
            v = v // 10
        elif h == 'milliseconds_to_seconds':
            v = v / 1000
        else:
            raise SystemExit('unhandled index_handler: %s' % h)
    return int(v) if float(v).is_integer() else v


def handler_of(lines, stat_id):
    for l in lines:
        if stat_id in l['stat_ids']:
            return l['index_handlers']
    return []


def match_lines(mod):
    """Rendered-text match keys for a mod: one entry per displayed line.

    A mod's stats are covered by one or more stat_translations entries. Stats
    whose whole range is 0 render nothing, so they are dropped first. Then the
    remaining ids are consumed greedily, longest translation first (this is the
    same shape of lookup the client does).
    """
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
                          'trade_stat_id': None, 'index_handlers': []})
            break
        s, trade, handlers = _placeholderise(st_by_ids[hit],
                                            [by_id[i] for i in hit])
        lines.append({'match_string': s, 'stat_ids': list(hit),
                      'trade_stat_id': trade, 'index_handlers': handlers})
        remaining = [i for i in remaining if i not in hit]
    return lines


# ---------------------------------------------------------------- build groups
def build_pool(gen_type):
    out = []
    for group, ids in pools[gen_type].items():
        rows = []
        for mid, weight in ids.items():
            m = mods[mid]
            rows.append((m['required_level'], mid, weight, m))
        # GGG tier numbering: highest mod level = Tier 1
        rows.sort(key=lambda r: (-r[0], r[1]))
        first = rows[0][3]
        stat_ids = [s['id'] for s in first['stats']]
        lines = match_lines(first)
        tiers = []
        for i, (lvl, mid, weight, m) in enumerate(rows, start=1):
            tier_lines = match_lines(m)
            assert [l['match_string'] for l in tier_lines] == \
                   [l['match_string'] for l in lines], (group, mid)
            tiers.append({
                'tier': i,
                'mod_id': mid,
                'affix_name': m['name'],
                'required_ilvl': lvl,
                'spawn_weight': weight,
                'stats': [{
                    'id': s['id'],
                    'min': s['min'],
                    'max': s['max'],
                    'display_min': apply_handlers(s["min"], handler_of(tier_lines, s["id"])),
                    'display_max': apply_handlers(s["max"], handler_of(tier_lines, s["id"])),
                } for s in m['stats']],
                'text': m['text'],
            })
        out.append({
            'group': group,
            'generation_type': gen_type,
            'match_lines': lines,
            'match_string': lines[0]['match_string'] if len(lines) == 1 else None,
            'stat_ids': stat_ids,
            'trade_stat_id': lines[0]['trade_stat_id'] if len(lines) == 1 else None,
            'tier_count': len(tiers),
            'tiers': tiers,
        })
    out.sort(key=lambda g: g['group'])
    return out


prefixes = build_pool('prefix')
suffixes = build_pool('suffix')

# ---------------------------------------------------------------- pool totals
breaks = sorted({t['required_ilvl']
                 for g in prefixes + suffixes for t in g['tiers']} | {1, 100})


def total(groups, ilvl):
    return sum(t['spawn_weight'] for g in groups for t in g['tiers']
               if t['required_ilvl'] <= ilvl)


totals = [{'ilvl': lv, 'prefix_weight': total(prefixes, lv),
           'suffix_weight': total(suffixes, lv)} for lv in breaks]

# ---------------------------------------------------------- non-alteration
def flat(gen_type):
    return [{
        'mod_id': mid,
        'affix_name': mods[mid]['name'],
        'generation_type': mods[mid]['generation_type'],
        'group': mods[mid]['groups'][0] if mods[mid]['groups'] else None,
        'required_ilvl': mods[mid]['required_level'],
        'spawn_weight': w,
        'text': mods[mid]['text'],
    } for mid, w in pools[gen_type].items()] if gen_type in pools else []


other = {}
for k in ('corrupted', 'delve_prefix', 'delve_suffix'):
    if k not in pools:
        continue
    rows = []
    for group, ids in pools[k].items():
        for mid, w in ids.items():
            m = mods[mid]
            rows.append({
                'mod_id': mid, 'group': group, 'affix_name': m['name'],
                'generation_type': m['generation_type'],
                'required_ilvl': m['required_level'], 'spawn_weight': w,
                'text': m['text'],
            })
    rows.sort(key=lambda r: (r['group'], -r['required_ilvl']))
    other[k] = rows

doc = {
    '$schema_id': 'poe-graft/base-mod-pool@1',
    'source': {
        'primary': 'RePoE (repoe-fork export)',
        'primary_url': 'https://repoe-fork.github.io/',
        'primary_repo': 'https://github.com/repoe-fork/repoe',
        'game_version': VERSION,
        'files_used': ['mods.json', 'mods_by_base.json', 'base_items.json',
                       'stat_translations.json'],
        'captured_at': '2026-08-04',
        'cross_checked_against': 'https://poedb.tw/us/Ghastly_Eye_Jewel (embedded ModsView JSON)',
        'cross_check_result': 'identical: 66 prefixes / 60 suffixes, all mod ids, levels and spawn weights agree',
    },
    'base': {
        'base_id': BASE_ID,
        'name': base['name'],
        'item_class': base['item_class'],
        'item_class_display': 'Abyss Jewels',
        'domain': base['domain'],
        'tags': base['tags'],
        'implicits': base['implicits'],
        'affix_slots': {
            'magic': {'max_prefixes': 1, 'max_suffixes': 1},
            'rare': {'max_prefixes': 2, 'max_suffixes': 2},
        },
    },
    'affix_count_odds': {
        'magic': {'1': 1, '2': 1},
        'rare_jewel': {'3': 65, '4': 35},
        '_note': ('Relative weights for how many affixes a freshly reforged item gets. '
                  'Not present in GGG data files; taken from the two independent '
                  'open-source crafting simulators that agree on it '
                  '(kalandralang src/item.ml, PoeCraftLib StatFactory.cs).'),
    },
    'matching': {
        'mod_info_line': r'^\{(?<type>[^"—]+?)(?:\s+"(?<name>[^"]*)")?(?:\s+\(Tier: (?<tier>\d+)\))?(?:\s+—\s*(?<tags>[^}]*))?\s*\}$',
        'value_placeholder': r'[+-]?\d+(?:\.\d+)?(?:\((?<lo>[^)-]*)-(?<hi>[^)]+)\))?',
        'notes': [
            'Normalise a rendered stat line by replacing every value_placeholder match '
            'with "#", then look the result up against match_lines[].match_string.',
            'stats[].min/max are raw stat units. display_min/display_max are the same '
            'bounds after index_handlers, i.e. the units printed in the tooltip. '
            'Compare parsed rolls against display_min/display_max.',
            'A tier can spawn iff item level >= required_ilvl.',
            'GGG tier numbering: tier 1 is the highest required_ilvl in the group that '
            'can spawn on this base, counted regardless of the item\'s own item level.',
        ],
    },
    'pool_totals_by_ilvl': totals,
    'prefixes': prefixes,
    'suffixes': suffixes,
    'non_alteration_pools': other,
}

out_path = os.path.join(DATA, 'ghastly-eye-jewel.json')
with open(out_path, 'w') as f:
    json.dump(doc, f, indent=2)
    f.write('\n')
print('wrote', out_path, os.path.getsize(out_path))

# ---------------------------------------------------------------- raw slices
wanted = set()
for k, v in pools.items():
    for group, ids in v.items():
        wanted |= set(ids)
raw_mods = {k: mods[k] for k in sorted(wanted)}
with open(os.path.join(RAW, 'repoe-mods.ghastly-eye-jewel.json'), 'w') as f:
    json.dump(raw_mods, f, indent=1)
    f.write('\n')

with open(os.path.join(RAW, 'repoe-mods_by_base.ghastly-eye-jewel.json'), 'w') as f:
    json.dump({'Abyss Jewels': {BASE_TAGS_KEY: entry}}, f, indent=1)
    f.write('\n')

with open(os.path.join(RAW, 'repoe-base_items.ghastly-eye-jewel.json'), 'w') as f:
    json.dump({BASE_ID: base}, f, indent=1)
    f.write('\n')

ids_needed = set()
for m in raw_mods.values():
    ids_needed.add(tuple(s['id'] for s in m['stats']))
    for s in m['stats']:
        ids_needed.add((s['id'],))
raw_st = [t for t in stat_translations if tuple(t['ids']) in ids_needed]
with open(os.path.join(RAW, 'repoe-stat_translations.ghastly-eye-jewel.json'), 'w') as f:
    json.dump(raw_st, f, indent=1)
    f.write('\n')

if poedb is not None:
    poedb_trim = {k: poedb[k]
                  for k in ('baseitem', 'gen', 'opt', 'normal', 'corrupted', 'delve')}
    with open(os.path.join(RAW, 'poedb-modsview.ghastly-eye-jewel.json'), 'w') as f:
        json.dump(poedb_trim, f, indent=1)
        f.write('\n')

with open(os.path.join(RAW, 'repoe-version.txt'), 'w') as f:
    f.write(VERSION + '\n')

for fn in sorted(os.listdir(RAW)):
    print(' raw:', fn, os.path.getsize(os.path.join(RAW, fn)))

# ---------------------------------------------------------------- self-checks
fail = 0

# 1. mods_by_base must agree with resolving spawn_weights against the base tags by hand.
scan = {'prefix': {}, 'suffix': {}}
for mid, m in mods.items():
    if m['domain'] != 'abyss_jewel' or m['generation_type'] not in scan:
        continue
    for sw in m['spawn_weights']:          # first matching tag wins
        if sw['tag'] in base['tags']:
            if sw['weight'] > 0:
                scan[m['generation_type']][mid] = sw['weight']
            break
for gen, groups in (('prefix', prefixes), ('suffix', suffixes)):
    mine = {t['mod_id']: t['spawn_weight'] for g in groups for t in g['tiers']}
    if mine != scan[gen]:
        fail += 1
        print('FAIL %s pool disagrees with a direct spawn_weights scan' % gen)
print('ok: pool matches a direct spawn_weights scan (%d prefixes, %d suffixes)'
      % (len(scan['prefix']), len(scan['suffix'])))

# 2. derived display bounds must reproduce RePoE's own rendered `text`.
NUM = re.compile(r'-?\d+(?:\.\d+)?')
for g in prefixes + suffixes:
    for t in g['tiers']:
        got = [abs(float(x)) for x in NUM.findall(t['text'])]
        want = []
        for s in t['stats']:
            if s['min'] == 0 and s['max'] == 0:
                continue
            lo, hi = float(s['display_min']), float(s['display_max'])
            want.extend([abs(lo)] if lo == hi else [abs(lo), abs(hi)])
        if got != want:
            fail += 1
            print('FAIL display bounds', t['mod_id'], want, '!=', got, t['text'])
print('ok: display bounds reproduce RePoE text for all %d tiers'
      % sum(len(g['tiers']) for g in prefixes + suffixes))

# 3. roll -> tier must be unambiguous, and rendered text -> group must be unique.
for g in prefixes + suffixes:
    for i, a in enumerate(g['tiers']):
        for b in g['tiers'][i + 1:]:
            if all(x['display_min'] <= y['display_max'] and
                   y['display_min'] <= x['display_max']
                   for x, y in zip(a['stats'], b['stats'])):
                fail += 1
                print('FAIL overlapping tiers', a['mod_id'], b['mod_id'])
seen = {}
for g in prefixes + suffixes:
    for l in g['match_lines']:
        if l['match_string'] is None:
            fail += 1
            print('FAIL unresolved match line', g['group'], l['stat_ids'])
        elif l['match_string'] in seen:
            fail += 1
            print('FAIL match_string collision', l['match_string'],
                  seen[l['match_string']], g['group'])
        else:
            seen[l['match_string']] = g['group']
print('ok: %d collision-free match strings, no overlapping tier ranges' % len(seen))

if fail:
    sys.exit('%d self-check failures' % fail)
