---
title: Dynamic Page Paths
description: How file paths become URLs
order: 3
tags:
  - basics
  - templates
---

### File path = URL

Your file's location is its address. Drop the `.md`, and that's the URL:

- `about.md` → `/about`
- `contact.md` → `/contact`
- `blog/config.md` → `/blog/config`

No routing config. No URL mapping. Put the file where you want the page.

### Index files

Files named `index.md` become the URL for their folder:

- `index.md` → `/` (homepage)
- `blog/index.md` → `/blog/`
- `docs/getting-started/index.md` → `/docs/getting-started/`

That's how you make section landing pages. The `blog/index.md` file you're reading from? It's the main blog page.

### Go as deep as you want

Nest folders however you like:

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

### One file, many pages

Say you're building a blog and want visitors to browse posts by topic. You'd need a page for each tag — `/blog/rust` shows all Rust posts, `/blog/web` shows web posts, and so on.

You could create each tag page by hand. But what happens when you add a new tag? Another file to maintain.

Hugs has a better way: **dynamic pages**. One template file generates multiple pages automatically.

Here's how it works. Create a file with brackets in the name:

```
blog/[tag].md
```

The brackets tell Hugs "this is a template." The name inside (`tag`) becomes a variable. Now in the frontmatter, list the values you want pages for:

{% raw %}
```markdown
---
title: Posts tagged {{ tag }}
tag:
  - rust
  - web
  - tutorial
---

{% for post in pages(within="/blog") %}
...filter posts by tag here...
{% endfor %}
```
{% endraw %}

Hugs generates three pages from this one file:
- `/blog/rust` (where `tag` = "rust")
- `/blog/web` (where `tag` = "web")
- `/blog/tutorial` (where `tag` = "tutorial")

The key: the bracket name (`[tag]`) must match a frontmatter field (`tag:`) that contains an array.

This pattern works for anything — author pages, category pages, year archives. One template, many pages.

One catch: brackets only work on filenames, not folders. `blog/[tag].md` works. `[category]/post.md` doesn't.

### See it in action

This site uses exactly this pattern. Each tutorial post has `tags` in its frontmatter, and `blog/[tag].md` generates a page for each topic:

[basics](/blog/basics) · [templating](/blog/templating) · [styling](/blog/styling) · [publishing](/blog/publishing)

Click any link — that's a dynamic page filtering posts by tag. One template, four URLs.

### Generate values with Jinja

You can use expressions too:

{% raw %}
```markdown
---
title: Page {{ page_no }}
page_no: "range(end=5)"
---
```
{% endraw %}

That creates `/1`, `/2`, `/3`, `/4`, `/5`. More on expressions in [Templating](/blog/templating).

### The special 404

`[404].md` is reserved. It doesn't generate dynamic pages — Hugs turns it into `404.html` for when visitors hit a missing page.

More on that in [Deployment](/blog/deployment).

{% call tryit() %}
1. Create a new file `docs.md` in your site root
2. Add a title in the frontmatter
3. Visit `/docs` — your new page is live
{% endcall %}

---