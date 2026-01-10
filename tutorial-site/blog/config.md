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

```toml
[site]
title = "My Hugs Site"
description = "A site built with Hugs"
url = "https://example.com"
author = "Your Name"
language = "en-us"           # Language code (default: "en-us")
twitter_handle = "@yourname" # For Twitter/X cards
default_image = "/og.png"    # Default image for social sharing
```

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
