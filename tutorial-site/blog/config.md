---
title: Config File
description: Site settings and metadata
order: 1
---

Every Hugs site has a `config.toml` file in its root folder. This is where you set site-wide settings like your site's title, description, and URL.

### Where It Lives

```
your-site/
├── config.toml    ← This file!
├── index.md
├── about.md
└── _/
    └── ...
```

### The Basics

Open your `config.toml` and you'll see something like this:

```toml
[site]
title = "My Hugs Site"
description = "A site built with Hugs"
url = "https://example.com"
author = "Your Name"
```

Each of these values gets used when building your site:

- **`title`** - Appears in browser tabs and search results
- **`description`** - Used in meta tags for SEO and social sharing
- **`url`** - Your site's full URL (important for feeds and social cards)
- **`author`** - Your name, used in meta tags and feeds

### Optional Fields

There are a few more fields you can add to the `[site]` section:

{% raw %}
```toml
[site]
title = "My Hugs Site"
description = "A site built with Hugs"
url = "https://example.com"
author = "Your Name"
language = "en-us"                              # Language code (default: "en-us")
twitter_handle = "@yourname"                    # For Twitter/X cards
default_image = "/og.png"                       # Default image for social sharing
title_template = "{{ title }} | {{ site.title }}" # Template for page titles
```
{% endraw %}

### Title Templates

By default, page titles are exactly what you put in the frontmatter. But often you want titles like "About | My Site" instead of just "About".

The `title_template` field lets you customize how titles are formatted:

{% raw %}
```toml
[site]
title = "My Site"
title_template = "{{ title }} | {{ site.title }}"
```
{% endraw %}

With this config, a page with `title: About` will display as "About | My Site" in:
- The browser tab (`<title>` tag)
- Social sharing cards (`og:title` and `twitter:title`)

Two variables are available in your template:
- **`title`** - The page's title from its frontmatter
- **`site.title`** - Your site's title from config.toml

Some example templates:

{% raw %}
```toml
# Site name after (most common)
title_template = "{{ title }} | {{ site.title }}"
# Result: "About | My Site"

# Site name before
title_template = "{{ site.title }} - {{ title }}"
# Result: "My Site - About"

# Just the page title (same as no template)
title_template = "{{ title }}"
# Result: "About"
```
{% endraw %}

### Build Settings

You can also configure how Hugs builds your site:

```toml
[build]
minify = true         # Minify HTML and CSS (default: true)
reading_speed = 200   # Words per minute for readtime() (default: 200)

[build.syntax_highlighting]
enabled = true           # Enable code highlighting (default: true)
theme = "one-dark-pro"   # Color theme for code blocks
```

### Using Config Values in Templates

You can access these values in your markdown files using template syntax:

```markdown
{% raw %}Welcome to {{ site.title }}!{% endraw %}
```

This makes it easy to reference your site's name without hardcoding it everywhere.

### Try It!

1. Open your `config.toml`
2. Change the `title` to something fun
3. Watch the browser tab update (live reload!)

---

Next up: [Pages & Frontmatter](/blog/pages-and-frontmatter) - learn how to configure individual pages.
