---
title: SEO & Meta Tags
description: How frontmatter maps to HTML meta tags
order: 9
tags:
  - publishing
---

### Search engines and social media, handled

Hugs generates SEO-friendly meta tags from your frontmatter and config automatically. Your pages are ready for search engines and social media without extra work.

### How it works

Set a title and description in frontmatter:

```markdown
---
title: My Awesome Post
description: Learn how to do something cool
---
```

Hugs generates the HTML — plus Open Graph tags (for Facebook, LinkedIn) and Twitter Card tags so your pages look great when shared:

```html
<title>My Awesome Post</title>
<meta name="description" content="Learn how to do something cool">
```

### What maps where

- **`title`** (frontmatter) → `<title>`, `og:title`, `twitter:title`
- **`description`** (frontmatter or site) → `<meta name="description">`, `og:description`, `twitter:description`
- **`author`** (frontmatter or site) → `<meta name="author">`
- **`image`** (frontmatter or default) → `og:image`, `twitter:image`
- **`url`** (site config) → `<link rel="canonical">`, `og:url`
- **`title`** (site config) → `og:site_name`
- **`twitter_handle`** (site config) → `twitter:site`

### Site-wide defaults

Set these once in `config.toml` and they apply everywhere:

```toml
[site]
title = "My Site"
description = "A blog about interesting things"
url = "https://mysite.com"
author = "Jane Doe"
twitter_handle = "@janedoe"
default_image = "/images/social-card.png"
```

`url` is important — it's used for canonical links and converting relative image paths to full URLs.

`default_image` provides a fallback social image for pages that don't specify one.

### Override per page

Page frontmatter always takes precedence:

```markdown
---
title: Guest Post
description: A special post with different metadata
author: Guest Author
image: /images/guest-post-cover.png
---
```

### Social share images

The `image` field controls what appears when your page is shared. Hugs checks in order:

1. Page's `image` frontmatter
2. Site's `default_image` config
3. If neither exists, no image tag

Relative paths become full URLs automatically. `/images/my-post.png` becomes `https://mysite.com/images/my-post.png`. External URLs work too.

### Twitter cards

Hugs picks the right card type:

- **`summary_large_image`** — when there's an image (large preview)
- **`summary`** — when there isn't (smaller card)

If you've set `twitter_handle` in config, it appears as `twitter:site`.

### Canonical URLs

Every page gets a `<link rel="canonical">` pointing to its full URL. This tells search engines which URL is the "official" version.

For `/blog/my-post` with `url = "https://mysite.com"`:

```html
<link rel="canonical" href="https://mysite.com/blog/my-post">
```

### Sitemap

Hugs generates `sitemap.xml` during builds — search engines use this to discover your pages. It includes every page with its canonical URL and `lastmod` dates if your pages have date fields.

No configuration needed, just make sure `url` is set.

### Everything that gets generated

Here's the full set of meta tags on every page:

```html
<!-- Basic SEO -->
<title>Page Title</title>
<meta name="description" content="...">
<meta name="author" content="...">
<link rel="canonical" href="https://mysite.com/page">

<!-- Open Graph (Facebook, LinkedIn) -->
<meta property="og:title" content="Page Title">
<meta property="og:description" content="...">
<meta property="og:url" content="https://mysite.com/page">
<meta property="og:type" content="website">
<meta property="og:image" content="https://mysite.com/images/...">
<meta property="og:site_name" content="My Site">

<!-- Twitter Cards -->
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="Page Title">
<meta name="twitter:description" content="...">
<meta name="twitter:image" content="https://mysite.com/images/...">
<meta name="twitter:site" content="@handle">
```

### Tips

- **Descriptions under 160 characters** — that's what shows in search results
- **Meaningful titles** — first thing people see, make it count
- **Set your site URL** — canonical links and images need it
- **Add social images** — posts with images get more engagement
- **Consistent author names** — set site-wide, override only for guests

{% call tryit() %}
1. Make sure `url` is set in `config.toml`
2. Add a `description` to any page
3. View source (Ctrl+U) and search for "og:"
4. Test with [Twitter's Card Validator](https://cards-dev.twitter.com/validator) or [Facebook's Sharing Debugger](https://developers.facebook.com/tools/debug/)
{% endcall %}

---