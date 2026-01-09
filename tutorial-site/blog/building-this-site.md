---
title: Building This Site
description: How this tutorial site was made
order: 11
---

You've been reading this tutorial site - now let's peek behind the curtain. This page explains how the site is structured, so you can apply the same ideas to your own projects.

### The File Structure

Here's the complete structure of this tutorial site:

```
docs/
├── config.toml          # Site settings
├── index.md             # Homepage - introduces page structure
├── about.md             # How to create pages and edit nav
├── blog/
│   ├── index.md         # Blog index with dynamic post listing
│   ├── config.md        # Tutorial: config.toml
│   ├── pages-and-frontmatter.md
│   ├── dynamic-paths.md
│   ├── templating.md
│   ├── syntax-highlighting.md
│   ├── theming.md
│   ├── assets.md
│   ├── seo.md
│   ├── deployment.md
│   ├── feeds.md
│   └── building-this-site.md  # You are here
└── _/
    ├── header.md        # Site header
    ├── nav.md           # Navigation links
    ├── footer.md        # Site footer
    └── theme.css        # Styling
```

### Tutorial Ordering

Each blog post has an `order` field in its frontmatter:

```markdown
---
title: Config File
order: 1
---
```

The blog index uses this to sort posts:

```
{% raw %}{% for post in pages(within="/blog") | sort(attribute="order") %}
- [{{ post.title }}]({{ post.url }})
{% endfor %}{% endraw %}
```

This keeps posts in the right sequence regardless of filenames or creation dates.

### CSS Styling

The `_/theme.css` file uses Hugs-specific attributes to target styling. Here are the key techniques:

**Showing titles only on blog posts:**

Hugs adds an `hg-title` attribute to page titles and an `hg-path` attribute to the body. This site hides titles by default and only shows them on blog posts:

```css
[hg-title] {
  display: none;
}

[hg-path^="blog "] [hg-title] {
  display: block;
}
```

The `hg-path^="blog "` selector matches any path starting with "blog " (note the space - this matches child pages, not just `/blog` itself).

**Compact list spacing on the blog index:**

The blog index shows a list of posts. To tighten up the spacing just for that page:

```css
[hg-path="blog"] li p {
  margin-bottom: 0.1em !important;
}
```

**Navigation as a horizontal row:**

The nav links come from `_/nav.md` as a paragraph of links. Flexbox turns them into a row:

```css
nav > p {
  display: flex;
  gap: 1.5rem;
}
```

**Sticky footer:**

On short pages, the footer sticks to the viewport bottom instead of floating mid-page:

```css
body {
  box-sizing: border-box;
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
}

main {
  flex: 1 0 auto;
}

footer {
  flex-shrink: 0;
}
```

**Base theme:**

The rest of `theme.css` is [Sakura.css](https://github.com/oxalorg/sakura/) - a minimal classless CSS framework. It styles plain HTML elements nicely without requiring classes.

### The Navigation Flow

Each tutorial post ends with a "Next up" link:

```markdown
---

Next up: [Pages & Frontmatter](/blog/pages-and-frontmatter) - learn about...
```

This creates a linear path through the content. Users can jump around via the blog index, but there's always a clear "what's next" for those following along.

### Accessing These Docs

This entire documentation site is embedded inside the Hugs binary itself. Running `hugs doc` extracts it to a temporary folder and serves it locally - no internet required!

This means you can reference the docs anytime, anywhere:

```bash
hugs doc              # Opens docs in your browser
hugs doc --port 9000  # Use a specific port
hugs doc --no-open    # Don't auto-open browser
```

The docs stay available as long as the command runs. Press `Ctrl+C` to stop.

### Your Turn

Hugs is designed to get out of your way. No complex build pipelines, no plugin ecosystems to navigate, no configuration rabbit holes. Just markdown files that become web pages.

This tutorial site is a starting point. Feel free to:

- **Delete the blog posts** and start fresh
- **Keep the structure** but replace the content
- **Use it as reference** while building something different

The best way to learn Hugs is to build something real. Pick a project - a blog, a portfolio, documentation for a side project - and start writing.

---

Thanks for following along! If you have questions or feedback, check out the [Hugs repository](https://github.com/AndrewBastin/hugs).

(っ◕‿◕)っ
