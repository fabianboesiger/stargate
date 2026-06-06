# Stargate landing page

Static, dependency-free landing page for downloading the Stargate desktop app.
Open `index.html` directly or serve the folder with any static host
(Netlify, Vercel, Cloudflare Pages, `python3 -m http.server`).

## Files

| File | Purpose |
| --- | --- |
| `index.html` | The page. Mirrors the app's design, with a light/dark toggle. |
| `styles.css` | Hand-authored theme (paper/graphite palette, Inter + mono labels). |
| `favicon.svg` | App "orbit" logo. |
| `og-image.svg` | Social share card (`og:image` / `twitter:image`). |
| `robots.txt` | Allows all crawlers, including AI bots; points to the sitemap. |
| `sitemap.xml` | Single-page sitemap. |
| `llms.txt` | Plain-language summary for AI assistants ([llmstxt.org](https://llmstxt.org)). |

## Before deploying

1. Replace every `CANONICAL_URL` placeholder (`https://stargate.example/`)
   with your real domain in `index.html`, `robots.txt` and `sitemap.xml`.
2. Optional: convert `og-image.svg` to a 1200×630 **PNG** named `og-image.png`
   and update the `og:image` / `twitter:image` tags, since some social
   scrapers don't render SVG previews.
3. Every download link points to the `downloads/` placeholder (search for
   `DOWNLOAD_URL` and `href="downloads/"`). Point these at your real installer
   or release page before going live.

## SEO / AI optimization included

- Title, description, keywords, canonical, `theme-color`, Open Graph & Twitter cards.
- JSON-LD structured data: `SoftwareApplication`, `Offer` (CHF), and `FAQPage`.
- Semantic HTML5 landmarks, accessible labels, `prefers-color-scheme` support.
- `robots.txt` explicitly welcomes GPTBot, ClaudeBot, PerplexityBot, etc.
- `llms.txt` gives AI assistants a clean, structured product summary.
