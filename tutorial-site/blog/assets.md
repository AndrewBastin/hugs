---
title: Assets & Static Files
description: Images, CSS, JavaScript, and other files
order: 7
---

Any file that isn't markdown is treated as a **static asset**. Images, fonts, PDFs, JavaScript files - they all get copied to your built site exactly as-is.

### Adding Images

Put image files **anywhere in your site directory** (except `_/`):

```
my-site/
├── images/
│   ├── logo.png
│   ├── hero.jpg
│   └── icons/
│       └── arrow.svg
├── index.md
└── about.md
```

Reference them in markdown with standard syntax:

```markdown
![My Logo](/images/logo.png)

![Hero image](/images/hero.jpg)
```

The path **starts with `/`** because it's relative to your site root.

### Organizing Assets

You can structure your assets however you like:

```
my-site/
├── images/           → /images/
├── fonts/            → /fonts/
├── downloads/        → /downloads/
│   └── resume.pdf    → /downloads/resume.pdf
└── scripts/          → /scripts/
    └── analytics.js  → /scripts/analytics.js
```

File paths in markdown **match the folder structure exactly**. A file at `images/photo.jpg` is accessed at `/images/photo.jpg`.

### Favicons

For a favicon, just put `favicon.ico` in your site's root directory:

```
my-site/
├── favicon.ico       → /favicon.ico
├── index.md
└── ...
```

Browsers **automatically request `/favicon.ico`**, and Hugs will serve it.

### Adding Custom CSS

The recommended approach is to edit `_/theme.css` directly. But if you prefer separate CSS files, you can:

1. Create a CSS file in your site directory (e.g., `styles/custom.css`)
2. Import it from `_/theme.css`:

```css
/* At the top of _/theme.css */
@import url('/styles/custom.css');

/* Rest of your theme... */
```

### Adding JavaScript

Embed JavaScript directly in your markdown using `<script>` tags:

```html
<script>
  console.log('Hello from my page!');
</script>
```

Or reference external scripts:

```html
<script src="/scripts/my-script.js"></script>
```

Since markdown in Hugs **supports HTML**, you can include scripts wherever you need them.

### Cache Busting

For production sites, you may want browser cache busting - ensuring browsers fetch new versions when files change.

Hugs provides a **`cache_bust()` template function** that adds a content hash to filenames:

{% raw %}
```html
<link rel="stylesheet" href="{{ cache_bust(path='/styles/custom.css') }}">
```
{% endraw %}

This outputs something like `/styles/custom.a1b2c3f4.css`. When the file content changes, the hash changes, and browsers **fetch the new version automatically**.

The built-in `theme.css` and `highlight.css` already use cache busting automatically.

### What Gets Copied

During `hugs build`, Hugs copies **all non-markdown files** to the output directory, preserving the folder structure. The exceptions:

- **Markdown files** (`.md`) - These become HTML pages
- **Files in `_/`** - These are structural files (theme, header, nav, footer)
- **`config.toml`** - Site configuration, not public

Everything else copies over unchanged.

### Linking to Assets in Frontmatter

You can reference asset paths in frontmatter for use in templates:

```markdown
---
title: My Post
image: /images/post-cover.jpg
---
```

The `image` field here is used for **social media previews** (Open Graph and Twitter cards). See [SEO & Meta Tags](/blog/seo) for details.

### Tips

- **Use absolute paths** starting with `/` for reliability across all pages
- **Keep images reasonable** - Hugs doesn't optimize images; use appropriately sized files
- **Organize by type or by page** - both work; pick what makes sense for your site
- **No special build step** - assets are copied as-is, no processing

### Try It!

1. Create an `images/` folder in your site
2. Add any image file (PNG, JPG, SVG, etc.)
3. Reference it in a markdown file: `![My image](/images/filename.jpg)`
4. See it appear on your page!

---

Next up: [SEO & Meta Tags](/blog/seo) - optimize your site for search engines and social sharing.
