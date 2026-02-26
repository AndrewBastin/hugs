---
title: Config File
description: Site settings and metadata
order: 1
tags:
  - basics
---

### Your site's control center

Every Hugs site starts with a `config.toml` file in the root folder. This is where your site gets its identity — the name, description, and all those little details that show up in browser tabs and search results.

```
your-site/
├── config.toml    ← Right here
├── index.md
├── about.md
└── _/
    └── ...
```

Open yours up. You'll see something like this:

{% raw %}
```toml
[site]
title = "Hugs Documentation"
description = "Learn how to build sites with Hugs"
url = "https://example.com"
author = "Hugs"
```
{% endraw %}

These four lines do a lot:

- **`title`** shows up in browser tabs and search results
- **`description`** appears in search previews and social shares
- **`url`** is your site's home address (needed for feeds and social cards)
- **`author`** is you

{% call tryit() %}
Change the `title` to something fun. Save. Watch the browser tab update.
{% endcall %}

> **Tip:** The `url` field matters for production — it's used to generate absolute URLs in RSS feeds and social meta tags. During local development, Hugs handles this automatically.

### A few more options

You can extend the `[site]` section with some extras:

{% raw %}
```toml
[site]
title = "My Hugs Site"
description = "A site built with Hugs"
url = "https://example.com"
author = "Your Name"
language = "en-us"                              # default
twitter_handle = "@yourname"                    # for Twitter/X cards
default_image = "/og.png"                       # fallback social image
title_template = "{{ title }} | {{ site.title }}" # how page titles look
# head_extra = '...'                             # raw HTML injected into <head>
```
{% endraw %}

### Adding extra tags to `<head>`

Need to add analytics, custom fonts, or other tags to `<head>`? Use `head_extra` to inject raw HTML into the `<head>` of every page:

```toml
[site]
head_extra = '<script src="https://example.com/analytics.js"></script>'
```

For multi-line snippets, use a TOML multi-line string:

```toml
[site]
head_extra = """
<script async src="https://www.googletagmanager.com/gtag/js?id=G-XXXXX"></script>
<script>
  window.dataLayer = window.dataLayer || [];
  function gtag(){dataLayer.push(arguments);}
  gtag('js', new Date());
  gtag('config', 'G-XXXXX');
</script>
"""
```

### Making page titles consistent

By default, a page's title is exactly what you set in frontmatter. But you probably want "About | My Site" instead of just "About".

That's what `title_template` does:

{% raw %}
```toml
[site]
title = "My Site"
title_template = "{{ title }} | {{ site.title }}"
```
{% endraw %}

Now a page with `title: About` shows as "About | My Site" — in the browser tab, in social cards, everywhere.

You have two variables to work with:
- **`title`** — the page's title from frontmatter
- **`site.title`** — your site title from config

A few patterns:

{% raw %}
```toml
# Site name after (most common)
title_template = "{{ title }} | {{ site.title }}"
# → "About | My Site"

# Site name first
title_template = "{{ site.title }} - {{ title }}"
# → "My Site - About"

# Just the page title
title_template = "{{ title }}"
# → "About"
```
{% endraw %}

### Build settings

There's also a `[build]` section for controlling how Hugs generates your site:

```toml
[build]
minify = true         # compress HTML and CSS (on by default)
reading_speed = 200   # words per minute for readtime()

[build.syntax_highlighting]
enabled = true           # code highlighting (on by default)
theme = "one-dark-pro"   # pick your color scheme
```

### Using config in your pages

You can pull these values into any page:

```markdown
{% raw %}Welcome to {{ site.title }}!{% endraw %}
```

Change the name once in config, and it updates everywhere.

---