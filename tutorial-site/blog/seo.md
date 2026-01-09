---
title: SEO & Meta Tags
description: How frontmatter maps to HTML meta tags
order: 8
---

Hugs **automatically generates SEO-friendly meta tags** from your frontmatter and config. Your pages are ready for search engines and social media without extra work.

### How It Works

When you set a title and description in frontmatter:

```markdown
---
title: My Awesome Post
description: Learn how to do something cool
---
```

Hugs generates the corresponding HTML:

```html
<title>My Awesome Post</title>
<meta name="description" content="Learn how to do something cool">
```

But it doesn't stop there - Hugs also generates Open Graph tags (for Facebook, LinkedIn) and Twitter Card tags so your pages look great when shared.

### The Full Picture

Here's how frontmatter and config fields map to meta tags:

- **`title`** (frontmatter) → `<title>`, `og:title`, `twitter:title`
- **`description`** (frontmatter or site) → `<meta name="description">`, `og:description`, `twitter:description`
- **`author`** (frontmatter or site) → `<meta name="author">`
- **`image`** (frontmatter or default) → `og:image`, `twitter:image`
- **`url`** (site config) → `<link rel="canonical">`, `og:url`
- **`title`** (site config) → `og:site_name`
- **`twitter_handle`** (site config) → `twitter:site`

### Site-Wide Defaults

Set defaults in `config.toml` that apply to all pages:

```toml
[site]
title = "My Site"
description = "A blog about interesting things"
url = "https://mysite.com"
author = "Jane Doe"
twitter_handle = "@janedoe"
default_image = "/images/social-card.png"
```

**`url`** is important - it's used for canonical links and converting relative image paths to full URLs.

**`default_image`** provides a fallback social image for pages that don't specify one.

### Page-Specific Overrides

Override any default in individual page frontmatter:

```markdown
---
title: Guest Post
description: A special post with different metadata
author: Guest Author
image: /images/guest-post-cover.png
---
```

Page values **always take precedence** over site defaults.

### Social Share Images

The `image` field controls what appears when your page is shared on social media. It works like this:

1. If the page has an `image` in frontmatter, use that
2. Otherwise, fall back to `default_image` from config
3. If neither exists, no image tag is generated

Relative paths are **converted to full URLs** automatically:

```markdown
---
image: /images/my-post.png
---
```

If your site URL is `https://mysite.com`, this becomes:
```html
<meta property="og:image" content="https://mysite.com/images/my-post.png">
```

External URLs work too:

```markdown
---
image: https://cdn.example.com/image.png
---
```

### Twitter Cards

Hugs automatically sets the right Twitter card type:

- **`summary_large_image`** - Used when your page has an image (displays a large preview)
- **`summary`** - Used when there's no image (displays a smaller card)

If you've set `twitter_handle` in config, it appears as the `twitter:site` tag.

### Canonical URLs

Every page gets a `<link rel="canonical">` tag pointing to its full URL. This helps search engines understand which URL is the "official" version of your page.

For a page at `/blog/my-post`, with `url = "https://mysite.com"`:

```html
<link rel="canonical" href="https://mysite.com/blog/my-post">
```

### Sitemap

Hugs automatically generates a `sitemap.xml` file during builds. This helps search engines discover all your pages.

The sitemap:
- Lists every page on your site
- Includes the full canonical URL for each page
- Adds `lastmod` dates if your pages have date fields

No configuration needed - just make sure `url` is set in your config.

### What Gets Generated

Here's the complete set of meta tags Hugs adds to every page:

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

### Best Practices

**Write good descriptions** - Keep them under 160 characters. They appear in search results and social previews.

**Use meaningful titles** - The title is the first thing people see. Make it clear and compelling.

**Set your site URL** - Without it, canonical links and image URLs won't work correctly.

**Add social images** - Posts with images get significantly more engagement when shared.

**Use consistent author names** - Set a site-wide author and only override for guest posts.

### Try It!

1. Open `config.toml` and make sure `url` is set
2. Add a `description` to any page's frontmatter
3. View source in your browser (Ctrl+U)
4. Search for "og:" to see the Open Graph tags
5. Paste a page URL into [Twitter's Card Validator](https://cards-dev.twitter.com/validator) or [Facebook's Sharing Debugger](https://developers.facebook.com/tools/debug/) to preview how it looks

---

Next up: [Deployment](/blog/deployment) - build for production and get your site online.
