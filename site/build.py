#!/usr/bin/env python3
"""Builds the Core Rules landing page into an output folder.

    python3 site/build.py _site
    python3 site/build.py _site --no-art   # offline preview, no paintings

Copies the page, the app's icons and the screenshots, and downloads the
paintings listed in art.json from Wikimedia Commons. A painting is only
used when Commons marks it as public domain; the credits on the page are
written from Commons' own metadata, so they always match the file used.
"""

import hashlib
import html
import json
import re
import shutil
import sys
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
API = "https://commons.wikimedia.org/w/api.php"
AGENT = "CoreRulesLandingPage/1.0 (https://github.com/brahm/corerules)"
FREE = re.compile(r"public domain|^pd\b|^pd-|cc0", re.I)


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": AGENT})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read()


def query(**params):
    params.update(format="json", formatversion="2")
    return json.loads(get(API + "?" + urllib.parse.urlencode(params)))


def text(value):
    value = re.sub(r"<[^>]+>", " ", value or "")
    return re.sub(r"\s+", " ", html.unescape(value)).strip()


def info(title, width):
    pages = query(action="query", titles=title, prop="imageinfo",
                  iiprop="url|extmetadata", iiurlwidth=width)["query"]["pages"]
    page = pages[0]
    if page.get("missing") or "imageinfo" not in page:
        return None
    return page["title"], page["imageinfo"][0]


def candidates(search):
    found = query(action="query", list="search", srnamespace=6,
                  srsearch=search, srlimit=8)["query"]["search"]
    return [f["title"] for f in found]


def fetch_art(out):
    (out / "art").mkdir(parents=True, exist_ok=True)
    credits, failed = [], False
    for art in json.loads((SITE / "art.json").read_text()):
        found = info(art["title"], art["width"])
        if not found:
            failed = True
            print(f"MISSING {art['title']}; Commons search for "
                  f"{art['search']!r}:")
            for title in candidates(art["search"]):
                hit = info(title, 400)
                meta = hit[1]["extmetadata"] if hit else {}
                print("   ", title, "|",
                      text(meta.get("LicenseShortName", {}).get("value")), "|",
                      text(meta.get("DateTimeOriginal", {}).get("value"))[:40])
            continue
        title, ii = found
        meta = ii["extmetadata"]
        licence = text(meta.get("LicenseShortName", {}).get("value"))
        if not FREE.search(licence):
            failed = True
            print(f"NOT FREE {title}: {licence!r}")
            continue
        (out / "art" / f"{art['slug']}.jpg").write_bytes(get(ii["thumburl"]))
        credit = {
            "slug": art["slug"],
            # Commons' ObjectName mixes languages and Wikidata markup.
            "name": art["name"],
            "artist": text(meta.get("Artist", {}).get("value")),
            # Commons appends Wikidata markup such as "date QS:P571,...".
            "date": text(meta.get("DateTimeOriginal", {}).get("value"))
            .split(" date QS:")[0],
            "licence": licence,
            "url": ii["descriptionurl"],
        }
        credits.append(credit)
        print("OK", title, "|", credit["name"], "|", credit["artist"], "|",
              credit["date"], "|", licence)
    if failed:
        sys.exit("Some paintings are missing or not free; see above.")
    return credits


def credit_list(credits):
    items = []
    for c in credits:
        by = f", {html.escape(c['artist'])}" if c["artist"] else ""
        when = f" ({html.escape(c['date'])})" if c["date"] else ""
        items.append(
            f'<li><a href="{html.escape(c["url"])}">'
            f"<cite>{html.escape(c['name'])}</cite></a>{by}{when}. "
            f"{html.escape(c['licence'])}, via Wikimedia Commons.</li>")
    return "\n".join(items)


def main():
    args = [a for a in sys.argv[1:] if a != "--no-art"]
    out = Path(args[0] if args else "_site").resolve()
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)

    shutil.copy(SITE / "style.css", out / "style.css")
    shutil.copy(SITE / "main.js", out / "main.js")
    shutil.copytree(SITE / "original", out / "original")
    shutil.copytree(SITE / "boxes", out / "boxes")
    shutil.copytree(SITE / "logo", out / "logo")
    shutil.copytree(SITE / "privacy", out / "privacy")
    shutil.copytree(ROOT / "docs" / "screenshots", out / "screenshots")
    shutil.copytree(ROOT / "addons" / "screenshots", out / "addons")
    icons = out / "icons"
    icons.mkdir()
    app = "data/icons/hicolor/{}/apps/io.github.brahm.CoreRules.png"
    shutil.copy(ROOT / app.format("512x512"), icons / "corerules.png")
    shutil.copy(ROOT / app.format("64x64"), icons / "favicon.png")
    shutil.copy(ROOT / "addons" / "icon.png", icons / "addons.png")
    (out / ".nojekyll").touch()

    page = (SITE / "index.html").read_text()
    # A browser that still holds the previous style sheet or script would
    # show the new page with it; naming their version in the link makes it
    # fetch the new ones.
    for asset in ("style.css", "main.js"):
        digest = hashlib.sha256((SITE / asset).read_bytes()).hexdigest()[:10]
        page = page.replace(f'"{asset}"', f'"{asset}?v={digest}"')
    credits = [] if "--no-art" in sys.argv else fetch_art(out)
    page = page.replace("<!--ART_CREDITS-->", credit_list(credits))
    (out / "index.html").write_text(page)
    print("Built", out)


if __name__ == "__main__":
    main()
