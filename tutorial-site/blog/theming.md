---
title: Theming & CSS
description: Customizing your site's look and feel
order: 7
tags:
  - styling
---

### The `_/` folder

Everything shared across pages lives here:

```
_/
├── theme.css   → your styles
├── header.md   → top of every page (logo, site name)
├── nav.md      → navigation links
├── content.md  → wraps page content (optional)
└── footer.md   → bottom of every page
```

These files don't become pages — they're injected into every page.

### Header, nav, footer

Three markdown files that show up everywhere:

**`_/header.md`** — the very top:
```markdown
**My Site** — A tagline here
```

**`_/nav.md`** — your navigation:
```markdown
[Home](/)
[About](/about)
[Blog](/blog)
```

**`_/footer.md`** — the bottom:
```markdown
Built with [Hugs](https://github.com/AdrianBastin/hugs)
```

All support markdown and HTML. Edit any of them — changes show up everywhere instantly.

### The content template

`_/content.md` wraps your page content. Without it, Hugs just renders your content directly. With it, you control the structure.

This site uses it to show titles only on blog posts, and to add the previous/next navigation at the bottom of each tutorial:

{% raw %}
```jinja
{% if path_class is startingwith("blog ") %}
# {{ title }}
{% endif %}

{{ content }}
```
{% endraw %}

Variables you can use: `content`, `title`, `path_class` (space-separated URL path like `blog macros`), plus any frontmatter fields.

Different layouts for different sections:

{% raw %}
```jinja
{% if path_class is startingwith("blog ") %}
<article>{{ content }}</article>
{% elif path_class is startingwith("docs ") %}
<div class="docs-content">{{ content }}</div>
{% else %}
{{ content }}
{% endif %}
```
{% endraw %}

### Page structure

Every page Hugs generates:

```html
<body hg-path="blog my-post">
  <header>...</header>
  <nav>...</nav>
  <main>...</main>
  <footer>...</footer>
</body>
```

The `hg-path` attribute is the URL path with slashes replaced by spaces. Use it for page-specific CSS:

```css
/* Homepage only */
[hg-path=""] main { text-align: center; }

/* All blog pages */
[hg-path^="blog"] { background-color: #f9f5f6; }

/* Specific page */
[hg-path="about"] h1 { color: #c9618a; }
```

### The default theme

Your site starts with a modified version of [Sakura](https://github.com/oxalorg/sakura/), a classless CSS theme with rose-tinted accents, the [Inter](https://rsms.me/inter/) font, and modern refinements. It styles HTML elements directly — no special classes needed.

The theme uses CSS variables — edit the `:root` block at the top of `_/theme.css`:

```css
:root {
  --color-primary: #b5507a;      /* links, accents */
  --color-background: #fefefe;   /* page background */
  --color-text: #1a1a1a;         /* body text */
  --color-text-muted: #555555;   /* secondary text */
  --color-secondary: #f9f5f6;    /* code blocks, cards */
  --color-border: #d4c4ca;       /* borders */
  --font-sans: "Inter", system-ui, sans-serif;
  --max-width: 42em;             /* content width */
}
```

Or override specific elements:

```css
/* Dark theme */
body { color: #e0e0e0; background-color: #1a1a1a; }
a { color: #f0a0c0; }

/* Wider content */
body { max-width: 50em; }

/* Different font */
html { font-family: Georgia, serif; }
```

### Code block styling

Syntax highlighting generates `/highlight.css` automatically. To style the blocks themselves:

```css
pre { border-radius: 8px; padding: 1.5em; }
code { font-family: 'Fira Code', monospace; }
```

### Tips

- **Browser dev tools** — inspect to see exact HTML structure
- **Check `hg-path`** — look at `<body>` to see path values
- **Live reload** — edit theme.css, see changes instantly

{% call tryit() %}
1. Open `_/theme.css`
2. Change `background-color` on `body`
3. Watch your browser update
{% endcall %}

---