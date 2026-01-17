---
title: RSS & Atom Feeds
description: Syndicate your content with RSS and Atom feeds
order: 11
tags:
  - publishing
---

### Let people subscribe

Feeds let visitors follow your site in their favorite reader — Feedly, NetNewsWire, Thunderbird, or any app that supports RSS. Hugs generates both **RSS 2.0** and **Atom** formats.

### Quick setup

Add this to `config.toml`:

```toml
[[feeds]]
name = "blog"
source = "/blog"
output_rss = "feed.xml"
```

That's it. Hugs creates `feed.xml` with all pages under `/blog/`.

The double brackets `[[feeds]]` mean you can define multiple feeds for different sections.

### RSS or Atom?

Both do the same thing. RSS is more recognized, Atom is more rigorous. Most readers support both, so you can generate both:

```toml
[[feeds]]
name = "blog"
source = "/blog"
output_rss = "feed.xml"
output_atom = "atom.xml"
```

### All the options

```toml
[[feeds]]
name = "blog"                          # identifier (required)
source = "/blog"                       # which pages to include (required)
output_rss = "feed.xml"                # RSS filename
output_atom = "atom.xml"               # Atom filename
title = "My Blog Feed"                 # defaults to site title
description = "Latest posts from..."   # defaults to site description
limit = 20                             # max items (default: 20)
```

At least one of `output_rss` or `output_atom` is required.

### How source filtering works

`source = "/blog"` includes:
- `/blog/my-first-post`
- `/blog/another-post`
- `/blog/2024/january-update`

But excludes:
- `/blog/` (the index page)
- `/about`
- `/projects/something`

Different feeds for different sections:

```toml
[[feeds]]
name = "blog"
source = "/blog"
output_rss = "blog.xml"

[[feeds]]
name = "tutorials"
source = "/tutorials"
output_rss = "tutorials.xml"

[[feeds]]
name = "everything"
source = "/"
output_rss = "all.xml"
limit = 50
```

### Dates matter

For feeds to sort correctly, posts need dates:

```markdown
---
title: My Post
description: A great post
date: 2024-06-15
---
```

Hugs recognizes `date`, `published`, `created`, or `pubDate`. Formats:

```yaml
date: 2024-06-15
date: 2024-06-15T10:30:00Z
date: 2024-06-15 10:30:00
```

Most recent posts appear first. Posts without dates go to the end.

### What goes in each feed item

Hugs pulls from your frontmatter:

| Feed field | Frontmatter source |
|------------|-------------------|
| Title | `title` (or "Untitled") |
| Link | page URL |
| Date | `date`, `published`, `created`, or `pubDate` |
| Description | `description`, `summary`, or `excerpt` |
| Author | `author` (or site author) |

A well-structured post:

```markdown
---
title: Getting Started with Hugs
description: Learn how to build your first static site
date: 2024-06-15
author: Jane Doe
---
```

### Site URL is required

Feeds need absolute URLs. Make sure this is set:

```toml
[site]
url = "https://mysite.com"
```

Without it, you'll get an error.

### Link to your feeds

Let visitors find them:

```markdown
[RSS](/feed.xml) | [Atom](/atom.xml)
```

Or with HTML `<link>` tags (for auto-discovery):

```html
<link rel="alternate" type="application/rss+xml" title="RSS" href="/feed.xml">
<link rel="alternate" type="application/atom+xml" title="Atom" href="/atom.xml">
```

### Test your feed

After building, validate with the [W3C Feed Validator](https://validator.w3.org/feed/).

Common issues:
- **Missing URL** — set `url` in config
- **Missing dates** — add `date` to frontmatter
- **Empty feed** — check that `source` matches your URLs

Try subscribing in a reader like [Feedly](https://feedly.com/), [NetNewsWire](https://netnewswire.com/), or [Inoreader](https://www.inoreader.com/) to see how it looks.

### Tips

- **Keep descriptions short** — 1-2 sentences, that's what readers show
- **Consistent dates** — pick a format, stick with it
- **Don't over-limit** — 20 items is usually right

{% call tryit() %}
1. Add a feed to `config.toml`
2. Make sure `url` is set in `[site]`
3. Add dates to your posts
4. Run `hugs build .` and check `dist/feed.xml`
{% endcall %}

---
