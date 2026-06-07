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

## Deployment

Deployed via GitHub Pages (`.github/workflows`) to **https://stargate-client.com/**.
The custom domain is set by the `CNAME` file in this folder; the canonical host is
also set in `index.html`, `robots.txt` and `sitemap.xml`; download links point at the
GitHub releases page. If the domain ever changes, update `CNAME` plus those three files
and `llms.txt`.

Optional: convert `og-image.svg` to a 1200×630 **PNG** named `og-image.png` and update
the `og:image` / `twitter:image` tags, since some social scrapers don't render SVG previews.

## SEO / AI optimization included

- Title, description, keywords, canonical, `theme-color`, Open Graph & Twitter cards.
- JSON-LD structured data: `SoftwareApplication`, `Offer` (CHF), and `FAQPage`.
- Semantic HTML5 landmarks, accessible labels, `prefers-color-scheme` support.
- `robots.txt` explicitly welcomes GPTBot, ClaudeBot, PerplexityBot, etc.
- `llms.txt` gives AI assistants a clean, structured product summary.
