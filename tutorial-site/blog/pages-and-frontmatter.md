---
title: Pages & Frontmatter
description: The --- blocks at the top of files
order: 2
---

Every markdown file in Hugs becomes a page on your site - with one exception. Files in the `_/` folder are special: they contain shared elements (header, nav, footer, theme) that appear on every page, not pages themselves. We'll cover those in [Theming & CSS](/blog/theming).

For regular pages, each file starts with a special block called **frontmatter**. It's that bit between the `---` markers at the very top of each file.

### What It Looks Like

```markdown
---
title: My Page Title
description: A brief description of this page
---

# Your content starts here...
```

The frontmatter uses YAML syntax - simple `key: value` pairs. Everything between the two `---` lines is metadata about the page. Everything after is your content.

### Built-in Fields

Hugs recognizes these frontmatter fields:

- **`title`** (required) - The page title. Shows in browser tabs, search results, and social shares.

- **`description`** (optional) - A brief summary. Used in meta tags for SEO and when people share your link on social media.

- **`author`** (optional) - The page author. Overrides the site-wide author from `config.toml` for this specific page.

- **`image`** (optional) - An image URL for social sharing. When someone shares your page on Twitter or Facebook, this image appears in the preview card.

### Example with All Fields

```markdown
---
title: My Awesome Post
description: This post explains something cool
author: Jane Doe
image: /images/awesome-post-cover.png
---

Your content here...
```

### Custom Fields

Here's the fun part - you can add **any fields you want** to frontmatter. Hugs passes them all through to templates.

For example, blog posts in this tutorial use an `order` field:

```markdown
---
title: Config File
description: Site settings and metadata
order: 1
---
```

You could use custom fields for dates, tags, categories, authors - whatever your site needs. We'll cover how to access these in templates in the [Templating](/blog/templating) post.

### How Fields Become Meta Tags

Your frontmatter automatically generates proper HTML meta tags:

- **`title`** → `<title>`, `og:title`, `twitter:title`
- **`description`** → `<meta name="description">`, `og:description`
- **`author`** → `<meta name="author">`
- **`image`** → `og:image`, `twitter:image`

This means your pages are SEO-ready and look great when shared on social media - no extra work needed. For more details on SEO and Open Graph tags, see the [SEO & Meta Tags](/blog/seo) post.

### Try It!

1. Open `about.md` in your site
2. Add a `description` field to the frontmatter
3. View source in your browser (Ctrl+U) and search for "description"
4. See your text in the `<meta>` tags!

---

Next up: [Dynamic Page Paths](/blog/dynamic-paths) - learn how file paths become URLs and how to create dynamic pages.
