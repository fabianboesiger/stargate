# Stargate website

Jekyll site for https://stargate-client.com/: the landing page, the SpacetimeDB guides, and
the files search engines and AI assistants read (sitemap, robots.txt, llms.txt).

## Local preview

```sh
jekyll serve -s site            # from the repository root, then open http://localhost:4000
jekyll build -s site -d _site && python3 scripts/check_site.py _site
```

CI builds with GitHub's `actions/jekyll-build-pages` (Jekyll 3.10, github-pages gem set), so keep
templates compatible with Jekyll 3. For an exact local match:

```sh
docker run --rm -v "$PWD/site":/src:ro -v "$PWD/_site":/out --entrypoint "" \
  ghcr.io/actions/jekyll-build-pages:v1.0.13 jekyll build -s /src -d /out
```

## Layout

| Path | Purpose |
| --- | --- |
| `_config.yml` | Site URL, author, download/GitHub/license links, hero screenshot size. |
| `_layouts/default.html` | Page shell: head, header, footer, theme toggle, OS-aware download links. |
| `_layouts/guide.html` | Guide article: breadcrumb, answer-first lede, FAQ, related guides, CTA. |
| `_includes/head.html` | Title, description, canonical, Open Graph/Twitter, preloads. |
| `_includes/jsonld-*.html` | Structured data per page type (home, guide, guides index, FAQ). |
| `_includes/guides/*.md` | Guide bodies. Shared by the guide pages and `llms-full.txt`. |
| `_includes/llms-summary.md` | Product summary shared by `llms.txt` and `llms-full.txt`. |
| `_data/faq.yml` | Every FAQ entry, grouped by page. Renders both the visible list and FAQPage JSON-LD. |
| `_data/icons.yml` | Inline Lucide icons (`{% include icon.html name="table" %}`). |
| `index.html` | Landing page. |
| `guides/*.md` | Guide front matter (title, description, lede, card text, order, dates). |
| `guides/index.html` | Guide overview. |
| `sitemap.xml`, `robots.txt`, `llms.txt`, `llms-full.txt` | Generated from templates. |
| `images/`, `fonts/`, `og-image.png` | Hero screenshots (WebP), self-hosted Inter (SIL OFL), social card. |

## Common edits

- **New guide:** add `_includes/guides/<slug>.md` with the body and `guides/<slug>.md` with front
  matter (copy an existing one, set `order`, `permalink`, `last_modified_at`). It appears in the
  nav cards, footer, sitemap, `llms.txt`, and `llms-full.txt` automatically.
- **Edit a page:** bump its `last_modified_at`, which feeds the sitemap and the "Updated" line.
- **FAQ:** edit `_data/faq.yml`. Each question belongs to exactly one page (`page:` group).
- **Icon:** copy the child elements of `lucide-static/icons/<name>.svg` into `_data/icons.yml`.
- **Social card:** edit `og-image.svg`, then re-render `og-image.png` at 1200x630.
- **Domain change:** update `CNAME`, `url` in `_config.yml`, and `SITE_URL` in `scripts/check_site.py`.

## Deployment

`.github/workflows/page.yml` builds the site, runs `scripts/check_site.py` (fails on invalid
JSON-LD, missing titles/descriptions/canonicals, empty icons, or sitemap gaps), and deploys to
GitHub Pages on every push to `main`.
