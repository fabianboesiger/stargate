#!/usr/bin/env python3
"""Render the social share cards (og:image, 1200x630 PNG) for the website.

Usage: python3 scripts/render_og.py

Every page under site/ whose front matter sets `og_image` gets a card rendered from
scripts/og/card.html into that path. The headline is `og_headline`, falling back to `h1`;
the eyebrow is `og_eyebrow`, falling back to "Guide · SpacetimeDB". Run it after changing
a page's headline and commit the PNGs; scripts/check_site.py fails when a card is missing.

Needs Google Chrome (set CHROME to another Chromium binary if needed). Only uses the
standard library. Front matter is read with a minimal parser for single-line `key: value`
entries, which is all these keys use.
"""

import html
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SITE = REPO / "site"
TEMPLATE = REPO / "scripts" / "og" / "card.html"
FONT = SITE / "fonts" / "inter-latin.woff2"
ICONS = SITE / "_data" / "icons.yml"
DEFAULT_EYEBROW = "Guide · SpacetimeDB"
FOOTER = "stargate-client.com · macOS · Windows · Linux"
CHROME = os.environ.get("CHROME", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")


def front_matter(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return {}
    block = text.split("---\n", 2)[1]
    values = {}
    for line in block.splitlines():
        m = re.match(r"^([a-z0-9_]+):\s*(.+)$", line)
        if m and not m.group(2).startswith(">"):
            values[m.group(1)] = m.group(2).strip().strip('"')
    return values


def orbit_icon() -> str:
    m = re.search(r"^orbit: '(.*)'$", ICONS.read_text(encoding="utf-8"), re.M)
    assert m, "orbit icon missing from _data/icons.yml"
    return m.group(1)


def render(card_html: str, out: Path, workdir: Path):
    page = workdir / "card.html"
    page.write_text(card_html, encoding="utf-8")
    out.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars",
         "--force-device-scale-factor=1", "--window-size=1200,630",
         "--virtual-time-budget=5000", f"--screenshot={out}", page.as_uri()],
        check=True, capture_output=True,
    )
    assert out.exists() and out.stat().st_size > 0, f"Chrome did not write {out}"


def main() -> int:
    template = TEMPLATE.read_text(encoding="utf-8")
    icon = orbit_icon()
    pages = [p for p in sorted(SITE.rglob("*")) if p.suffix in (".html", ".md")
             and not any(part.startswith("_") for part in p.relative_to(SITE).parts)]
    rendered = 0
    with tempfile.TemporaryDirectory() as tmp:
        workdir = Path(tmp)
        shutil.copy(FONT, workdir / FONT.name)
        for path in pages:
            fm = front_matter(path)
            if "og_image" not in fm:
                continue
            headline = fm.get("og_headline") or fm.get("h1")
            assert headline, f"{path}: og_image is set but neither og_headline nor h1 is"
            card = (template
                    .replace("__ORBIT__", icon)
                    .replace("__EYEBROW__", html.escape(fm.get("og_eyebrow", DEFAULT_EYEBROW)))
                    .replace("__HEADLINE__", html.escape(headline))
                    .replace("__FOOTER__", html.escape(FOOTER)))
            out = SITE / fm["og_image"].lstrip("/")
            render(card, out, workdir)
            rendered += 1
            print(f"render_og: {path.relative_to(SITE)} -> {out.relative_to(SITE)}")
    assert rendered, "no page sets og_image"
    print(f"render_og: rendered {rendered} cards")
    return 0


if __name__ == "__main__":
    sys.exit(main())
