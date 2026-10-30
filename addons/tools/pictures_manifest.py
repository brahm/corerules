"""Chooses where each add-on picture goes.

usage: pictures_manifest.py EXTRAS_DIR REFS.tsv ITEMS.tsv CD_BITMAPS_DIR OUT_DIR

EXTRAS_DIR      the unzipped Comunidade-extras.zip
REFS.tsv        the picture paths the records of Comunidade.xpx name (written by
                the cr-addons `pictures` example, removed with the Linux
                installer; see the git history)
ITEMS.tsv       order, package, category, name, hash of every add-on item
CD_BITMAPS_DIR  the CD's BITMAPS folder, whose pictures are never replaced
"""
import collections, hashlib, os, sys

extras, refs_tsv, items_tsv, cd, out = sys.argv[1:6]
EVERMORE = ('official addons/', 'equipment/cr2-march-goodie/')

def evermore(path):
    p = path.lower()
    return p.startswith(EVERMORE) or 'goodie' in p

files = []
for d, _, fs in os.walk(extras):
    for f in fs:
        rel = os.path.relpath(os.path.join(d, f), extras).replace(os.sep, '/')
        if not evermore(rel):
            files.append(rel)
files.sort()
bmps = [f for f in files if f.lower().endswith('.bmp')]
digest = {b: hashlib.sha1(open(os.path.join(extras, b), 'rb').read()).hexdigest() for b in bmps}
by_name = collections.defaultdict(list)
for b in bmps:
    by_name[b.rsplit('/', 1)[-1].lower()].append(b)

cd_files = set()
for d, _, fs in os.walk(cd):
    for f in fs:
        cd_files.add(('bitmaps/' + os.path.relpath(os.path.join(d, f), cd)).replace(os.sep, '/').lower())

# The package each final item came from: the newest one that has it.
winner = {}
for line in open(items_tsv, encoding='utf-8'):
    order, pkg, cat, name, _ = line.rstrip('\n').split('\t')
    pkg = pkg.removeprefix('./')
    key = (cat, name)
    if key not in winner or int(order) >= winner[key][0]:
        winner[key] = (int(order), pkg)

def closeness(a, b):
    a, b = a.lower().split('/')[:-1], b.lower().split('/')[:-1]
    n = 0
    while n < min(len(a), len(b)) and a[n] == b[n]:
        n += 1
    return n

targets = collections.defaultdict(collections.Counter)
spelling = {}
unmatched = skipped_cd = 0
for line in open(refs_tsv, encoding='utf-8'):
    cat, name, stored, rel = line.rstrip('\n').split('\t')
    cands = by_name.get(rel.rsplit('/', 1)[-1].lower())
    if not cands:
        unmatched += 1
        continue
    if rel.lower() in cd_files:
        skipped_cd += 1
        continue
    pkg = winner.get((cat, name), (0, ''))[1]
    best = max(cands, key=lambda c: (closeness(c, pkg), -cands.index(c)))
    spelling.setdefault(rel.lower(), rel)
    targets[rel.lower()][best] += 1

placements, clashes = [], 0
for rel, counter in sorted(targets.items()):
    (src, _), *rest = counter.most_common()
    if any(digest[s] != digest[src] for s, _ in rest):
        clashes += 1
    placements.append((src, spelling[rel]))

os.makedirs(out, exist_ok=True)
with open(os.path.join(out, 'placements.tsv'), 'w', encoding='utf-8') as f:
    for src, rel in placements:
        f.write(f'{src}\t{rel}\n')
with open(os.path.join(out, 'files.txt'), 'w', encoding='utf-8') as f:
    f.write('\n'.join(files) + '\n')
used = {s for s, _ in placements}
print(f'{len(files)} files, {len(bmps)} pictures, {len(used)} placed for records '
      f'in {len(placements)} places, {len(bmps) - len(used)} only in UserBMP; '
      f'{unmatched} references with no picture, {skipped_cd} to CD pictures, '
      f'{clashes} places where different pictures share a name')
