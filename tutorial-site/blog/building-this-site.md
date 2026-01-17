---
title: Building This Site
description: How this tutorial site was made
order: 12
tags:
  - basics
---

### Behind the curtain

You've been reading this tutorial — now let's look at how it's built. Same techniques work for your own projects.

### File structure

```
tutorial-site/
├── config.toml
├── index.md
├── about.md
├── blog/
│   ├── index.md              # lists all posts
│   ├── [tag].md              # dynamic tag pages
│   ├── config.md
│   ├── pages-and-frontmatter.md
│   ├── ...
│   └── building-this-site.md # you are here
└── _/
    ├── header.md
    ├── nav.md
    ├── footer.md
    ├── content.md            # wraps page content
    ├── theme.css
    └── macros/
        └── tryit.md          # "Try it yourself" boxes
```

### Post ordering

Each post has an `order` field:

```markdown
---
title: Config File
order: 1
---
```

The blog index sorts by it:

{% raw %}
```jinja
{% for post in pages(within="/blog") | sort(attribute="order") %}
- [{{ post.title }}]({{ post.url }})
{% endfor %}
```
{% endraw %}

Posts stay in sequence regardless of filename or creation date.

### The tryit macro

You've seen the "Try it yourself" boxes throughout. That's a macro in `_/macros/tryit.md`:

{% raw %}
```jinja
{% call tryit() %}
1. Open config.toml
2. Change something
{% endcall %}
```
{% endraw %}

One file, reusable everywhere. See [Macros](/blog/macros) for how to create your own.

### Tags for blog posts

Tags are rendered as clickable badges that link to tag pages:

{% raw %}
```jinja
{% for tag in tags %}
  <a href="/blog/{{ tag }}" class="tag-badge">{{ tag }}</a>
{% endfor %}
```
{% endraw %}

### Prev/next navigation

Notice the cards at the bottom of each tutorial? Those come from `_/content.md`. It looks up posts by their `order` field and renders navigation links:

{% raw %}
```jinja
{% set all_posts = pages(within="/blog") | selectattr("order") | list %}
{% set prev_post = all_posts | selectattr("order", "eq", order - 1) | first %}
{% set next_post = all_posts | selectattr("order", "eq", order + 1) | first %}

{% if prev_post %}
  <!-- render prev card -->
{% endif %}
{% if next_post %}
  <!-- render next card -->
{% endif %}
```
{% endraw %}

No manual "Next up" links needed — the navigation generates itself from the `order` values.

### CSS techniques

**Titles only on blog posts** — `_/content.md` checks `path_class`:

{% raw %}
```jinja
{% if path_class is startingwith("blog ") %}
# {{ title }}
{% endif %}
```
{% endraw %}

**Horizontal nav** — flexbox on the links:

```css
nav > p {
  display: flex;
  gap: 1.5rem;
}
```

**Sticky footer** — flex column layout:

```css
body {
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
}
main { flex: 1 0 auto; }
footer { flex-shrink: 0; }
```

**Base theme** — a modified version of [Sakura.css](https://github.com/oxalorg/sakura/) with [Inter](https://rsms.me/inter/) font, rose-tinted accents, and CSS variables for easy customization.

---