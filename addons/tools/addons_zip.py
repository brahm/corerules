"""Builds the add-ons' zip, CoreRules-Community-Addons-VERSION.zip: the
validated add-ons as files that Core Rules' Import Add-ons reads from a
folder.

usage: addons_zip.py VERSION OUT_DIR

Reads addons/Comunidade.xpx.xz and addons/Comunidade-extras.xz (see
extras_bundle.py for the extras' format) and writes, under
one folder, CoreRules-Community-Addons-VERSION/:
    Comunidade.xpx      the consolidated add-ons, in the Expansion's format
    pictures/...        the pictures, in their add-on folders
    pictures.tsv        picture<TAB>place in the installation, one per line
    texts/...           the authors' texts, as plain text
    addons.tsv          what each add-on folder brought
    README.txt, LEIA-ME.txt
"""
import lzma, os, sys, zipfile

here = os.path.dirname(os.path.abspath(__file__))
addons = os.path.dirname(here)
version, out_dir = sys.argv[1:3]
name = f'CoreRules-Community-Addons-{version}'

data = lzma.open(os.path.join(addons, 'Comunidade-extras.xz')).read()
end = data.index(b'\n\n')
lines = data[:end].decode('utf-8').split('\n')
if lines[0] != 'CRX1':
    sys.exit('Comunidade-extras.xz: not a CRX1 bundle')
at, files, placements = end + 2, [], []
for line in lines[1:]:
    kind, a, b = line.split('\t')
    if kind == 'F':
        files.append((b, data[at:at + int(a)]))
        at += int(a)
    elif kind == 'P':
        placements.append(f'{a}\t{b}\n')
if at > len(data):
    sys.exit('Comunidade-extras.xz: truncated')

def readme(lang):
    with open(os.path.join(here, f'addons_zip.{lang}.txt'), encoding='utf-8') as f:
        return f.read().replace('VERSION', version).replace('\n', '\r\n').encode('utf-8')

# Fixed dates and order, so the same input gives the same zip.
def put(z, path, raw, compress=zipfile.ZIP_DEFLATED):
    info = zipfile.ZipInfo(f'{name}/{path}', date_time=(2000, 1, 1, 0, 0, 0))
    info.external_attr = 0o644 << 16
    z.writestr(info, raw, compress_type=compress, compresslevel=9)

os.makedirs(out_dir, exist_ok=True)
out = os.path.join(out_dir, f'{name}.zip')
with zipfile.ZipFile(out + '.part', 'w') as z:
    put(z, 'README.txt', readme('en'))
    put(z, 'LEIA-ME.txt', readme('pt'))
    put(z, 'Comunidade.xpx', lzma.open(os.path.join(addons, 'Comunidade.xpx.xz')).read())
    put(z, 'pictures.tsv', ''.join(placements).encode('utf-8'))
    for path, raw in files:
        put(z, path, raw)
os.replace(out + '.part', out)
print(out)
