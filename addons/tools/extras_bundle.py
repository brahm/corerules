"""Builds addons/Comunidade-extras.xz: the add-ons' pictures and texts,
and what each add-on folder brought.

usage: extras_bundle.py EXTRAS_DIR MANIFEST_DIR ITEMS.tsv FINAL_ITEMS.tsv COMPAT.csv WORK_DIR OUT

The bundle, before xz, is a text header then the files' bytes in order:
    CRX1
    F<TAB>size<TAB>path        a file: pictures/..., texts/....txt, addons.tsv
    P<TAB>picture<TAB>place     where a picture goes, in the installation
    (blank line)
addons.tsv, one block per add-on folder:
    A<TAB>folder
    K<TAB>package<TAB>items in the package
    I<TAB>category<TAB>name      the items, under their package
    T<TAB>texts/....txt<TAB>original file name
"""
import collections, csv, html, html.parser, os, re, subprocess, sys

extras, manifest, items_tsv, final_tsv, compat_csv, work, out = sys.argv[1:8]
files = [l.rstrip('\n') for l in open(os.path.join(manifest, 'files.txt'), encoding='utf-8') if l.strip()]
placements = [l.rstrip('\n').split('\t') for l in open(os.path.join(manifest, 'placements.tsv'), encoding='utf-8')]
pictures = [f for f in files if f.lower().endswith('.bmp')]
texts = [f for f in files if f.lower().rsplit('.', 1)[-1] in ('txt', 'htm', 'html', 'doc', 'rtf', 'pdf')]

class Text(html.parser.HTMLParser):
    BLOCK = {'p', 'br', 'div', 'tr', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'table', 'hr'}
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.out, self.skip = [], 0
    def handle_starttag(self, tag, attrs):
        if tag in ('script', 'style', 'title'): self.skip += 1
        if tag in self.BLOCK: self.out.append('\n')
    def handle_endtag(self, tag):
        if tag in ('script', 'style', 'title'): self.skip = max(0, self.skip - 1)
        if tag in self.BLOCK: self.out.append('\n')
    def handle_data(self, data):
        if not self.skip: self.out.append(re.sub(r'\s+', ' ', data))

def decode(raw):
    # DOS text with box-drawing art (Fantaseum readmes) is code page 437.
    high = [b for b in raw if b >= 0x80]
    if len([b for b in high if 0xb0 <= b <= 0xdf]) > max(20, len(high) // 2):
        try: raw.decode('utf-8')
        except UnicodeDecodeError: return raw.decode('cp437')
    for enc in ('utf-8', 'cp1252'):
        try: return raw.decode(enc)
        except UnicodeDecodeError: pass
    return raw.decode('latin-1')

def tidy(text):
    text = text.replace('\r\n', '\n').replace('\r', '\n').lstrip('﻿')
    text = '\n'.join(l.rstrip() for l in text.split('\n'))
    return re.sub(r'\n{3,}', '\n\n', text).strip() + '\n'

conv = os.path.join(work, 'conv')
os.makedirs(conv, exist_ok=True)
converted = {}
for t in texts:
    src = os.path.join(extras, t)
    ext = t.lower().rsplit('.', 1)[-1]
    if ext == 'txt':
        text = decode(open(src, 'rb').read())
    elif ext in ('htm', 'html'):
        p = Text(); p.feed(decode(open(src, 'rb').read())); text = ''.join(p.out)
    else:
        dest = os.path.join(conv, re.sub(r'[^A-Za-z0-9]+', '_', t) + '.txt')
        if not os.path.exists(dest):
            if ext == 'pdf':
                subprocess.run(['pdftotext', '-layout', '-enc', 'UTF-8', src, dest], check=True)
            else:
                tmp = os.path.join(work, 'soffice'); os.makedirs(tmp, exist_ok=True)
                subprocess.run(['soffice', '--headless', '--convert-to', 'txt:Text (encoded):UTF8',
                                '--outdir', tmp, src], check=True, capture_output=True, timeout=300)
                os.replace(os.path.join(tmp, os.path.basename(src).rsplit('.', 1)[0] + '.txt'), dest)
        text = open(dest, encoding='utf-8', errors='replace').read()
    text = tidy(text)
    if text.strip():
        converted[t] = text

# Packages and the items each brought into the final package.
rows = [r for r in csv.DictReader(open(compat_csv, encoding='utf-8'))
        if not (r['addon'].lower().startswith('official addons/') or 'goodie' in r['addon'].lower())]
final = {tuple(l.rstrip('\n').split('\t')[:2]) for l in open(final_tsv, encoding='utf-8')}
winner = {}
for line in open(items_tsv, encoding='utf-8'):
    order, pkg, cat, name, _ = line.rstrip('\n').split('\t')
    key = (cat, name)
    if key in final and (key not in winner or int(order) >= winner[key][0]):
        winner[key] = (int(order), pkg.removeprefix('./'))
brought = collections.defaultdict(list)
for (cat, name), (_, pkg) in winner.items():
    brought[pkg].append((cat, name))

folders = collections.defaultdict(list)
total = {r['addon']: int(r['itens'] or 0) for r in rows}
for r in rows:
    folders[r['addon'].rsplit('/', 1)[0] if '/' in r['addon'] else ''].append(r['addon'])
def home(path):
    d = path.rsplit('/', 1)[0]
    best = max((f for f in folders if d == f or d.startswith(f + '/')), key=len, default=None)
    if best is None:
        under = [f for f in folders if f.startswith(d + '/')]
        best = under[0] if len(under) == 1 else d
    return best
folder_texts = collections.defaultdict(list)
for t in converted:
    folder_texts[home(t)].append(t)

index = []
for folder in sorted(set(folders) | set(folder_texts), key=str.lower):
    index.append(f'A\t{folder}')
    for pkg in sorted(folders.get(folder, []), key=str.lower):
        items = sorted(brought.get(pkg, []))
        index.append(f'K\t{pkg}\t{total[pkg]}')
        index += [f'I\t{c}\t{n}' for c, n in items]
    for t in sorted(folder_texts.get(folder, []), key=str.lower):
        index.append(f'T\ttexts/{t}.txt\t{t.rsplit("/", 1)[-1]}')
index = ('\n'.join(index) + '\n').encode('utf-8')

entries = [('addons.tsv', index)]
entries += [(f'texts/{t}.txt', converted[t].encode('utf-8')) for t in sorted(converted)]
entries += [(f'pictures/{p}', open(os.path.join(extras, p), 'rb').read()) for p in pictures]
header = ['CRX1'] + [f'F\t{len(d)}\t{p}' for p, d in entries]
header += [f'P\tpictures/{s}\t{t}' for s, t in placements]
blob = ('\n'.join(header) + '\n\n').encode('utf-8') + b''.join(d for _, d in entries)
raw = os.path.join(work, 'extras.bin')
open(raw, 'wb').write(blob)
subprocess.run(['xz', '-9e', '-T1', '--check=crc64', '-kf', raw], check=True)
os.replace(raw + '.xz', out)
folders_count = sum(1 for l in index.split(b'\n') if l.startswith(b'A'))
print(f'{len(pictures)} pictures, {len(converted)} texts of {len(texts)}, '
      f'{folders_count} add-on folders, '
      f'{sum(len(v) for v in brought.values())} items; {len(blob)} bytes, {os.path.getsize(out)} compressed')
