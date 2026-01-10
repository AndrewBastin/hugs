---
title: Theming & CSS
description: Customizing your site's look and feel
order: 6
---

Your site's appearance is controlled by files in the `_/` folder. This special folder contains everything that's **shared across all pages** - your theme, header, navigation, and footer.

### The `_/` Folder

```
_/
├── theme.css   → Your site's styles
├── header.md   → Content above the nav (logo, site name)
├── nav.md      → Navigation links
├── content.md  → Content area template (optional)
└── footer.md   → Content at the bottom of every page
```

Files in this folder don't become pages themselves. Instead, they're **injected into every page** your site generates.

### Header, Nav, and Footer

These three markdown files control the persistent elements of your site:

**`_/header.md`** - Appears at the very top of every page. Great for your site logo or name:

```markdown
<div style="display: flex; justify-content: space-between;">
  <strong>My Site</strong>
  <span>A tagline here</span>
</div>
```

**`_/nav.md`** - Your navigation links, appearing below the header:

```markdown
[Home](/)
[About](/about)
[Blog](/blog)
```

Just write markdown links - they render inline as your navigation menu.

**`_/content.md`** - Controls how the main content area is rendered. This is optional - if not present, Hugs just renders your page content directly. But with it, you can wrap content with custom markup, conditionally show titles, or add consistent elements to all pages.

**`_/footer.md`** - Appears at the bottom of every page:

```markdown
<center>
  Built with [Hugs](https://github.com/AndrewBastin/hugs)
</center>
```

All three files support both markdown and HTML. Edit them and see changes on every page instantly.

### The Content Template

The `_/content.md` file deserves special attention. It's a **Jinja template** that controls how your page content is rendered within the `<main>` element.

If you don't have a `_/content.md` file, Hugs defaults to simply rendering:

{% raw %}
```jinja
{{ content }}
```
{% endraw %}

But you can customize it to add structure around your content. Here's what this tutorial site uses:

{% raw %}
```jinja
{% if path_class is startingwith("blog ") %}
# {{ title }}
{% endif %}

{{ content }}
```
{% endraw %}

This shows the page title as an `<h1>` only on blog posts (pages whose path starts with "blog "), but not on other pages like the homepage or about page.

**Available variables in `_/content.md`:**

- `content` - The rendered HTML content of your page
- `title` - The page title from frontmatter
- `path_class` - Space-separated URL path (e.g., `blog macros` for `/blog/macros`)
- `base` - Base path for relative URLs
- `seo` - SEO context object with `canonical_url`, `og_title`, etc.
- All custom frontmatter fields from the page (like `order`, `tags`, etc.)

**Example: Different layouts for different sections**

{% raw %}
```jinja
{% if path_class is startingwith("blog ") %}
<article>
# {{ title }}
{{ content }}
</article>
{% elif path_class is startingwith("docs ") %}
<div class="docs-content">
{{ content }}
</div>
{% else %}
{{ content }}
{% endif %}
```
{% endraw %}

**Example: Add consistent elements to all pages**

{% raw %}
```jinja
{{ content }}

---
*Last updated: {{ date | default("Unknown") }}*
```
{% endraw %}

### The HTML Structure

Every page Hugs generates has the same structure:

```html
<body hg-path="blog my-post">
  <header>...</header>
  <nav>...</nav>
  <main>
    <!-- content from _/content.md template -->
  </main>
  <footer>...</footer>
</body>
```

This gives you clear hooks for styling: `header`, `nav`, `main`, `footer`, and the content within them.

### The `hg-path` Attribute

Hugs adds the `hg-path` attribute to `<body>`. This is a space-separated version of the page's URL path, so `/blog/my-post` becomes `blog my-post`.

Why space-separated instead of the actual path? CSS attribute selectors treat spaces as word boundaries. This means you can use `[hg-path~="blog"]` to match any page that has "blog" as a path segment, rather than relying on substring matching which could produce false positives.

### Conditional Styling with `hg-path`

The `hg-path` attribute enables page-specific styling using CSS attribute selectors:

```css
/* Style only the homepage */
[hg-path=""] main {
  text-align: center;
}

/* Style all blog pages (path starts with "blog") */
[hg-path^="blog"] {
  background-color: #fafafa;
}

/* Style a specific page */
[hg-path="about"] h1 {
  color: #1d7484;
}
```

### The Default Theme

Your site comes with [Sakura](https://github.com/oxalorg/sakura/), a minimal classless CSS theme. It styles standard HTML elements without requiring any special classes.

Key customization points:

```css
/* Main colors */
body {
  color: #4a4a4a;           /* Text color */
  background-color: #f9f9f9; /* Background */
}

a {
  color: #1d7484;           /* Link color */
}

a:hover {
  color: #982c61;           /* Link hover */
}

/* Typography */
html {
  font-size: 62.5%;         /* Base size (makes rem calc easy) */
}

body {
  font-size: 1.8rem;        /* Content text size */
  line-height: 1.618;       /* Golden ratio line height */
  max-width: 38em;          /* Content width */
}
```

### Common Customizations

**Change the color scheme:**

```css
body {
  color: #e0e0e0;
  background-color: #1a1a1a;
}

a { color: #6db3f2; }
a:visited { color: #b794f4; }
a:hover { color: #f687b3; }
```

**Adjust content width:**

```css
body {
  max-width: 50em;  /* Wider content area */
}
```

**Change fonts:**

```css
html {
  font-family: Georgia, serif;
}

h1, h2, h3, h4, h5, h6 {
  font-family: 'Helvetica Neue', sans-serif;
}
```

**Style the navigation:**

```css
nav > p {
  display: flex;
  gap: 1.5rem;
}

nav a {
  font-weight: 600;
}
```

**Sticky footer:**

```css
body {
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
}

main { flex: 1 0 auto; }
footer { flex-shrink: 0; }
```

### Adding Custom CSS

The `_/theme.css` file is the only CSS file Hugs loads automatically. To add more styles:

1. **Edit `_/theme.css` directly** - simplest approach, keep everything in one file
2. **Use CSS `@import`** - at the top of theme.css: `@import url('custom.css');`

### Syntax Highlighting Styles

If syntax highlighting is enabled, Hugs generates a separate `/highlight.css` file for code block colors. This is automatically included in your pages. To customize code block appearance beyond colors, add styles to your theme:

```css
pre {
  border-radius: 8px;
  padding: 1.5em;
}

code {
  font-family: 'Fira Code', monospace;
}
```

### Tips

- **Use browser dev tools** - Right-click and "Inspect" to see exactly what HTML Hugs generates
- **Check `hg-path` values** - Inspect the `<body>` tag to see the exact path value for any page
- **CSS is minified** - Don't worry about file size; Hugs minifies CSS in production builds
- **Live reload works** - Edit theme.css and see changes instantly

### Try It!

1. Open `_/theme.css` in your editor
2. Find the `body` rule and change `background-color`
3. Watch your browser update instantly
4. Try adding a custom `_/content.md` that wraps your content in an `<article>` tag

---

Next up: [Assets & Static Files](/blog/assets) - learn how to include images, CSS, and JavaScript.
