#!/usr/bin/env python3
"""Check the built website (Jekyll output) for SEO and structured-data mistakes.

Usage: python3 scripts/check_site.py <built-site-dir>

Fails (exit code 1) and lists every problem when:
- a JSON-LD block is not valid JSON, or a FAQPage does not match the visible FAQ,
- an inline icon is empty (unknown name in site/_data/icons.yml),
- a page lacks a title, description, canonical URL, or exactly one <h1>,
- Liquid tags leaked into the output,
- sitemap.xml, robots.txt, or llms.txt reference URLs that were not built,
- an indexable page is missing from sitemap.xml.
Uses only the standard library so it runs on a bare CI runner.
"""

import json
import re
import sys
import xml.etree.ElementTree as ET
from html.parser import HTMLParser
from pathlib import Path

SITE_URL = "https://stargate-client.com"
TITLE_MAX = 70
DESCRIPTION_RANGE = (50, 170)


class PageParser(HTMLParser):
    """Collects the bits of a page the checks need."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.title = ""
        self.meta = {}
        self.canonical = None
        self.h1_count = 0
        self.jsonld_blocks = []
        self.faq_details = 0
        self.empty_icons = []
        self._in_title = False
        self._in_jsonld = False
        self._jsonld_buf = []
        self._faq_depth = 0
        self._icon = None  # (name, has_children)

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "title":
            self._in_title = True
        elif tag == "meta" and "name" in a:
            self.meta[a["name"]] = a.get("content", "")
        elif tag == "link" and a.get("rel") == "canonical":
            self.canonical = a.get("href")
        elif tag == "h1":
            self.h1_count += 1
        elif tag == "script" and a.get("type") == "application/ld+json":
            self._in_jsonld = True
            self._jsonld_buf = []
        elif tag == "div" and "faq" in (a.get("class") or "").split():
            self._faq_depth = 1
        elif tag == "div" and self._faq_depth:
            self._faq_depth += 1
        elif tag == "details" and self._faq_depth:
            self.faq_details += 1
        elif tag == "svg" and "data-icon" in a:
            self._icon = [a["data-icon"], False]
        elif self._icon is not None:
            self._icon[1] = True

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        if self._icon is not None and tag != "svg":
            self._icon[1] = True

    def handle_endtag(self, tag):
        if tag == "title":
            self._in_title = False
        elif tag == "script" and self._in_jsonld:
            self._in_jsonld = False
            self.jsonld_blocks.append("".join(self._jsonld_buf))
        elif tag == "div" and self._faq_depth:
            self._faq_depth -= 1
        elif tag == "svg" and self._icon is not None:
            if not self._icon[1]:
                self.empty_icons.append(self._icon[0])
            self._icon = None

    def handle_data(self, data):
        if self._in_title:
            self.title += data
        if self._in_jsonld:
            self._jsonld_buf.append(data)


def url_for(path: Path, root: Path) -> str:
    rel = path.relative_to(root).as_posix()
    if rel.endswith("index.html"):
        rel = rel[: -len("index.html")]
    return f"{SITE_URL}/{rel}"


def built_path(url: str, root: Path) -> Path:
    assert url.startswith(SITE_URL), url
    rel = url[len(SITE_URL):].lstrip("/").split("#")[0]
    if rel == "" or rel.endswith("/"):
        rel += "index.html"
    return root / rel


def find_faq_pages(node):
    if isinstance(node, dict):
        if node.get("@type") == "FAQPage":
            yield node
        for v in node.values():
            yield from find_faq_pages(v)
    elif isinstance(node, list):
        for v in node:
            yield from find_faq_pages(v)


def check_page(path: Path, root: Path, errors: list) -> bool:
    """Checks one HTML page. Returns True when the page is indexable."""
    text = path.read_text(encoding="utf-8")
    rel = path.relative_to(root).as_posix()
    p = PageParser()
    p.feed(text)

    if re.search(r"\{\{|\{%", text):
        errors.append(f"{rel}: unrendered Liquid in output")
    if not p.title.strip():
        errors.append(f"{rel}: missing <title>")
    elif len(p.title) > TITLE_MAX:
        errors.append(f"{rel}: title is {len(p.title)} chars (max {TITLE_MAX}): {p.title!r}")
    desc = p.meta.get("description", "")
    if not DESCRIPTION_RANGE[0] <= len(desc) <= DESCRIPTION_RANGE[1]:
        errors.append(f"{rel}: description is {len(desc)} chars (want {DESCRIPTION_RANGE})")
    if p.h1_count != 1:
        errors.append(f"{rel}: has {p.h1_count} <h1> elements (want 1)")
    for name in p.empty_icons:
        errors.append(f"{rel}: icon {name!r} rendered empty (missing in _data/icons.yml?)")

    faq_questions = 0
    for i, block in enumerate(p.jsonld_blocks):
        try:
            data = json.loads(block)
        except json.JSONDecodeError as e:
            errors.append(f"{rel}: JSON-LD block {i} is invalid: {e}")
            continue
        for faq in find_faq_pages(data):
            faq_questions += len(faq.get("mainEntity", []))
    if faq_questions != p.faq_details:
        errors.append(f"{rel}: FAQPage has {faq_questions} questions, page shows {p.faq_details}")

    noindex = "noindex" in p.meta.get("robots", "")
    if noindex:
        return False
    expected = url_for(path, root)
    if p.canonical != expected:
        errors.append(f"{rel}: canonical {p.canonical!r}, expected {expected!r}")
    if not p.jsonld_blocks:
        errors.append(f"{rel}: no JSON-LD")
    return True


def check_text_links(name: str, root: Path, errors: list):
    path = root / name
    if not path.exists():
        errors.append(f"{name}: not built")
        return
    for url in re.findall(re.escape(SITE_URL) + r"[^\s)>\"']*", path.read_text(encoding="utf-8")):
        if not built_path(url, root).exists():
            errors.append(f"{name}: links to {url}, which was not built")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    root = Path(sys.argv[1]).resolve()
    errors: list = []

    indexable = set()
    pages = sorted(root.rglob("*.html"))
    for path in pages:
        if check_page(path, root, errors):
            indexable.add(url_for(path, root))

    sitemap = root / "sitemap.xml"
    in_sitemap = set()
    if sitemap.exists():
        ns = {"s": "http://www.sitemaps.org/schemas/sitemap/0.9"}
        for loc in ET.parse(sitemap).getroot().findall("s:url/s:loc", ns):
            in_sitemap.add(loc.text)
            if not built_path(loc.text, root).exists():
                errors.append(f"sitemap.xml: {loc.text} was not built")
    else:
        errors.append("sitemap.xml: not built")
    for url in sorted(indexable - in_sitemap):
        errors.append(f"sitemap.xml: indexable page {url} is missing")

    for name in ("robots.txt", "llms.txt", "llms-full.txt"):
        check_text_links(name, root, errors)
    if "Sitemap: " + SITE_URL + "/sitemap.xml" not in (root / "robots.txt").read_text(encoding="utf-8"):
        errors.append("robots.txt: missing Sitemap line")

    if errors:
        print(f"check_site: {len(errors)} problem(s) found")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(f"check_site: OK ({len(pages)} HTML pages, {len(indexable)} indexable, {len(in_sitemap)} in sitemap)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
