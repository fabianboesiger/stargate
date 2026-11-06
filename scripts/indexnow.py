#!/usr/bin/env python3
"""Submit every URL in the built sitemap to IndexNow (Bing, Yandex, Seznam, Naver, ...).

Usage: python3 scripts/indexnow.py <built-site-dir>

Run after the site is deployed: search engines fetch the key file
(/indexnow-key.txt, rendered from `indexnow_key` in site/_config.yml) from the live site
to verify the submission. Exits non-zero when IndexNow does not accept the request.
Uses only the standard library so it runs on a bare CI runner.
"""

import json
import re
import sys
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path
from urllib.parse import urlparse

ENDPOINT = "https://api.indexnow.org/indexnow"
KEY_FILE = "indexnow-key.txt"
# 200: URLs received. 202: received, key validation pending.
ACCEPTED = {200, 202}


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    root = Path(sys.argv[1])

    key = (root / KEY_FILE).read_text(encoding="utf-8").strip()
    assert re.fullmatch(r"[a-zA-Z0-9-]{8,128}", key), f"invalid IndexNow key in {KEY_FILE}"

    ns = {"s": "http://www.sitemaps.org/schemas/sitemap/0.9"}
    urls = [loc.text for loc in ET.parse(root / "sitemap.xml").getroot().findall("s:url/s:loc", ns)]
    assert urls, "sitemap.xml lists no URLs"
    hosts = {urlparse(u).netloc for u in urls}
    assert len(hosts) == 1, f"sitemap spans several hosts: {hosts}"
    host = hosts.pop()

    payload = {
        "host": host,
        "key": key,
        "keyLocation": f"https://{host}/{KEY_FILE}",
        "urlList": urls,
    }
    request = urllib.request.Request(
        ENDPOINT,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json; charset=utf-8"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            status = response.status
    except urllib.error.HTTPError as e:
        status = e.code
        print(f"indexnow: HTTP {status}: {e.read().decode('utf-8', 'replace')[:500]}")

    if status not in ACCEPTED:
        print(f"indexnow: submission of {len(urls)} URLs for {host} rejected (HTTP {status})")
        return 1
    print(f"indexnow: submitted {len(urls)} URLs for {host} (HTTP {status})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
