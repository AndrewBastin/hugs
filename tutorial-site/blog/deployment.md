---
title: Deployment
description: Building for production and hosting options
order: 9
---

You've been using `hugs dev` for local development. When you're ready to publish, `hugs build` creates optimized static files ready for any web host.

### Building for Production

Run the build command from your terminal:

```bash
hugs build my-site
```

This creates a `dist/` folder containing your complete static site:

```
dist/
├── index.html
├── about/
│   └── index.html
├── blog/
│   ├── index.html
│   ├── my-post/
│   │   └── index.html
│   └── ...
├── theme.css
├── highlight.css
├── sitemap.xml
├── images/
│   └── ...
└── ...
```

Every markdown file becomes an `index.html` in its own folder, giving you clean URLs like `/about` instead of `/about.html`.

### Custom Output Directory

By default, Hugs outputs to `dist/`. Use the `-o` flag to change it:

```bash
hugs build my-site -o public
hugs build my-site -o build
hugs build my-site -o _site
```

Different hosts expect different folder names - use whatever your host prefers.

### What the Build Does

During a production build, Hugs:

1. **Renders all pages** - Converts markdown to optimized HTML
2. **Minifies HTML & CSS** - Removes whitespace and comments for smaller files
3. **Generates sitemap.xml** - Helps search engines discover your pages
4. **Generates feeds** - Creates RSS/Atom feeds if configured
5. **Copies static assets** - Images, fonts, and other files
6. **Creates 404.html** - Custom error page if you have a `404.md`
7. **Cache-busts assets** - Adds content hashes for browser caching

### Build Configuration

Control build behavior in `config.toml`:

```toml
[build]
minify = true  # Default: true

[build.syntax_highlighting]
enabled = true           # Default: true
theme = "one-dark-pro"   # Default theme
```

Setting `minify = false` can be useful for debugging the generated HTML.

### Where to Host

Your built site is just static files - it works on any static hosting service:

**Free options:**
- **GitHub Pages** - Free for public repos, custom domains supported
- **Netlify** - Generous free tier, automatic HTTPS
- **Cloudflare Pages** - Fast global CDN, unlimited bandwidth
- **Vercel** - Great performance, easy setup
- **GitLab Pages** - Similar to GitHub Pages

**Traditional hosts:**
- Any web server (nginx, Apache, Caddy)
- S3 + CloudFront
- Any shared hosting with FTP access

### GitHub Pages

The simplest approach for GitHub repos. Add this workflow to `.github/workflows/deploy.yml`:

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

Then enable GitHub Pages in your repo settings, selecting "GitHub Actions" as the source.

### Netlify

Drop your `dist/` folder into Netlify's web UI, or connect your repo for automatic deploys.

For repo-connected deploys, create `netlify.toml`:

```toml
[build]
  command = "cargo install hugs && hugs build . -o dist"
  publish = "dist"
```

Or use the Netlify UI to set:
- **Build command:** `cargo install hugs && hugs build .`
- **Publish directory:** `dist`

### Cloudflare Pages

Connect your GitHub/GitLab repo and configure:
- **Build command:** `cargo install hugs && hugs build . -o dist`
- **Build output directory:** `dist`

Cloudflare Pages has excellent global performance and a generous free tier.

### Vercel

Create `vercel.json` in your repo:

```json
{
  "buildCommand": "cargo install hugs && hugs build . -o dist",
  "outputDirectory": "dist"
}
```

### Pre-built Binaries

For faster CI builds, download the Hugs binary directly instead of compiling:

```yaml
- name: Install Hugs
  run: |
    curl -L https://github.com/AndrewBastin/hugs/releases/latest/download/hugs-linux-x64 -o hugs
    chmod +x hugs
    sudo mv hugs /usr/local/bin/
```

This skips the Rust compilation step, making deploys much faster.

### Custom 404 Pages

Most static hosts serve `404.html` for missing pages. Create `404.md` in your site root:

```markdown
---
title: Page Not Found
---

# Oops!

The page you're looking for doesn't exist.

[Go back home](/)
```

Hugs automatically generates `404.html` during builds.

### Tips

**Test your build locally** - Before deploying, run `hugs build` and preview the result:

```bash
hugs build my-site
cd dist
python -m http.server 8000
```

**Set your site URL** - The `url` field in `config.toml` is used for canonical links and sitemap. Make sure it matches your deployed domain:

```toml
[site]
url = "https://mysite.com"
```

**Check your sitemap** - After deploying, visit `/sitemap.xml` to verify all pages are listed correctly.

**Submit to search engines** - Submit your sitemap URL to Google Search Console and Bing Webmaster Tools.

### Build Output Summary

After building, Hugs shows a summary:

```
INFO Build complete! 15 pages, 1 feeds, sitemap, 8 assets
```

If there are issues (like missing site URL for sitemap), you'll see warnings with helpful details.

---

Next up: [RSS & Atom Feeds](/blog/feeds) - syndicate your content with feeds.
