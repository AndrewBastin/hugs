---
title: Deployment
description: Building for production and hosting options
order: 10
tags:
  - publishing
---

### From dev to production

You've been running `hugs dev`. When you're ready to publish, `hugs build` creates optimized static files for any web host.

```bash
hugs build my-site
```

> **Important:** Before building, make sure `url` in your `config.toml` matches your production domain. This affects RSS feeds, sitemaps, and social meta tags.

This creates a `dist/` folder with your complete site:

```
dist/
├── index.html
├── about/
│   └── index.html
├── blog/
│   ├── index.html
│   └── my-post/
│       └── index.html
├── theme.css
├── highlight.css
├── sitemap.xml
└── images/
    └── ...
```

Every markdown file becomes an `index.html` in its own folder — clean URLs like `/about` instead of `/about.html`.

### Custom output directory

Default is `dist/`. Change it with `-o`:

```bash
hugs build my-site -o public
hugs build my-site -o _site
```

Use whatever your host expects.

### What the build does

1. **Renders all pages** — markdown to optimized HTML
2. **Minifies HTML & CSS** — smaller files, faster loads
3. **Generates sitemap.xml** — helps search engines find your pages
4. **Generates feeds** — RSS/Atom if configured
5. **Copies static assets** — images, fonts, everything else
6. **Creates 404.html** — if you have a `[404].md`
7. **Cache-busts assets** — content hashes for browser caching

### Build configuration

Control build behavior in `config.toml`:

```toml
[build]
minify = true  # Default: true

[build.syntax_highlighting]
enabled = true           # Default: true
theme = "one-dark-pro"   # Default theme
```

Set `minify = false` if you need to debug the generated HTML.

### Where to host

Static files work anywhere:

**Free options:**

| Host | Highlights |
|------|------------|
| [GitHub Pages](https://pages.github.com/) | Free for public repos, custom domains |
| [Netlify](https://www.netlify.com/) | Generous free tier, automatic HTTPS |
| [Cloudflare Pages](https://pages.cloudflare.com/) | Fast global CDN, unlimited bandwidth |
| [Vercel](https://vercel.com/) | Great performance, easy setup |
| [GitLab Pages](https://docs.gitlab.com/ee/user/project/pages/) | Similar to GitHub Pages |

**Traditional:** nginx, Apache, Caddy, S3 + CloudFront, any shared hosting

### GitHub Pages

Add `.github/workflows/deploy.yml`:

```yaml
name: Deploy to GitHub Pages

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pages: write
      id-token: write

    steps:
      - uses: actions/checkout@v4

      - name: Install Hugs
        run: cargo install hugs

      - name: Build site
        run: hugs build . -o dist

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: dist

      - name: Deploy to GitHub Pages
        uses: actions/deploy-pages@v4
```

Enable GitHub Pages in repo settings → select "GitHub Actions" as source.

### Netlify

Drop `dist/` into Netlify's web UI, or connect your repo. For automatic deploys, create `netlify.toml`:

```toml
[build]
  command = "cargo install hugs && hugs build . -o dist"
  publish = "dist"
```

### Cloudflare Pages

Connect your repo and set:
- **Build command:** `cargo install hugs && hugs build . -o dist`
- **Output directory:** `dist`

### Vercel

Create `vercel.json`:

```json
{
  "buildCommand": "cargo install hugs && hugs build . -o dist",
  "outputDirectory": "dist"
}
```

### Faster CI with pre-built binaries

Skip Rust compilation by downloading the binary directly:

```yaml
- name: Install Hugs
  run: |
    curl -L https://github.com/AndrewBastin/hugs/releases/latest/download/hugs-linux-x64 -o hugs
    chmod +x hugs
    sudo mv hugs /usr/local/bin/
```

Much faster deploys.

### Custom 404 page

Create `[404].md` in your site root:

```markdown
---
title: Page Not Found
---

# Oops!

The page you're looking for doesn't exist.

[Go back home](/)
```

Hugs generates `404.html` automatically. Most static hosts serve it for missing pages.

### Before you deploy

**Test locally:**

```bash
hugs build my-site
cd dist
python -m http.server 8000
```

**Set your site URL** — canonical links and sitemap need it:

```toml
[site]
url = "https://mysite.com"
```

**After deploying:** check `/sitemap.xml`, submit it to Google Search Console and Bing Webmaster Tools.

### Build output

Hugs shows a summary when done:

```
INFO Build complete! 15 pages, 1 feeds, sitemap, 8 assets
```

If something's off (like missing site URL), you'll see warnings with details.

---