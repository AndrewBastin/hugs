---
title: Dynamic Page Paths
description: How file paths become URLs
order: 3
---

Every markdown file you create becomes a page on your site. But how does Hugs decide what URL each file gets? It's simpler than you might think.

### The Basic Rule

Your file path **is** your URL, minus the `.md` extension:

- `about.md` → `/about`
- `contact.md` → `/contact`
- `blog/config.md` → `/blog/config`

That's it! **No routing config, no URL mapping** - just put the file where you want the page.

### Index Files

Files named `index.md` get special treatment - they **become the URL for their folder**:

- `index.md` → `/` (your homepage)
- `blog/index.md` → `/blog/`
- `docs/getting-started/index.md` → `/docs/getting-started/`

This lets you create section landing pages. The `blog/index.md` file in this tutorial is the main blog page that lists all posts.

### Nested Folders

Folders can go as deep as you need:

```
your-site/
├── index.md                      → /
├── about.md                      → /about
├── blog/
│   ├── index.md                  → /blog/
│   ├── first-post.md             → /blog/first-post
│   └── tutorials/
│       ├── index.md              → /blog/tutorials/
│       └── getting-started.md    → /blog/tutorials/getting-started
```

### Dynamic Pages

Here's where it gets interesting. Sometimes you want to generate **multiple pages from a single template** - like pages for each tag, author, or paginated lists.

Hugs uses **bracket notation** for dynamic pages. A file named `[slug].md` becomes a template that generates multiple pages.

For example, `blog/[tag].md` with this frontmatter:

{% raw %}
```markdown
---
title: Posts tagged {{ tag }}
tag:
  - rust
  - web
  - tutorial
---
```
{% endraw %}

This generates three pages:
- `/blog/rust`
- `/blog/web`
- `/blog/tutorial`

The parameter name inside the brackets (e.g., `tag`) **must match** a frontmatter field containing an array of values.

**Note:** Dynamic brackets only work on markdown files, not folders. You can have `blog/[tag].md`, but not `[category]/post.md`.

### Dynamic Values with Jinja

You can also use Jinja expressions to generate values:

{% raw %}
```markdown
---
title: Page {{ page_no }}
page_no: "range(end=5)"
---
```
{% endraw %}

This generates pages `/1`, `/2`, `/3`, `/4`, `/5` (or wherever the file lives).

For more on what you can do with Jinja expressions, see the [Templating](/blog/templating) post.

### Try It!

1. Create a new file `docs.md` in your site root
2. Add a title in the frontmatter
3. Visit `/docs` in your browser - your new page is live!

---

Next up: [Templating](/blog/templating) - learn how to use variables and logic in your pages.
