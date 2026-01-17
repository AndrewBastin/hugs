---
title: Pages & Frontmatter
description: The --- blocks at the top of files
order: 2
tags:
  - basics
---

### One file, one page

Every markdown file becomes a page on your site. The exception? Files in the `_/` folder — those are shared pieces (header, nav, footer) that show up everywhere. More on that in [Theming & CSS](/blog/theming).

At the top of each page file, you'll see a block between `---` markers. That's **frontmatter** — metadata about the page.

```markdown
---
title: My Page Title
description: A brief description of this page
---

# Your content starts here...
```

Simple `key: value` pairs. Everything between the dashes is about the page. Everything after is the page.

### What Hugs knows about

A few fields have special meaning:

- **`title`** — shows in browser tabs, search results, social shares. Required.
- **`description`** — the summary that appears in search previews and link shares.
- **`author`** — overrides the site-wide author for this page.
- **`image`** — the preview image when someone shares your link on social media.

Put them together:

```markdown
---
title: My Awesome Post
description: This post explains something cool
author: Jane Doe
image: /images/awesome-post-cover.png
---

Your content here...
```

### Make up your own

Here's where it gets interesting — you can add any field you want. Hugs passes everything through to templates.

The posts in this tutorial use an `order` field:

```markdown
---
title: Config File
description: Site settings and metadata
order: 1
---
```

Dates, tags, categories, reading time — whatever your site needs. How to use them? That's in [Templating](/blog/templating).

### From frontmatter to meta tags

Your fields automatically become proper HTML:

- **`title`** → `<title>`, `og:title`, `twitter:title`
- **`description`** → `<meta name="description">`, `og:description`
- **`author`** → `<meta name="author">`
- **`image`** → `og:image`, `twitter:image`

SEO-ready, social-share-friendly, no extra work. More on this in [SEO & Meta Tags](/blog/seo).

{% call tryit() %}
1. Open `about.md` in your site
2. Add a `description` field to the frontmatter
3. View source in your browser (Ctrl+U) and search for "description"
4. Your text is now in the `<meta>` tags
{% endcall %}

---