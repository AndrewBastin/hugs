---
title: Templating
description: Variables and logic in your markdown
order: 4
---

Your markdown files aren't just static text - they're **templates**. Hugs uses [Jinja](https://jinja.palletsprojects.com/) (via [MiniJinja](https://docs.rs/minijinja)), a powerful templating engine.

### Variables

Use double curly braces to output values:

{% raw %}
```jinja
The page title is: {{ title }}
```
{% endraw %}

### Available Variables

Every page has access to these built-in variables:

- `title` - The page title from frontmatter
- `path_class` - A CSS-friendly class based on the URL (e.g., `blog my-post`)
- `base` - The base URL path for the current page
- `url` - The URL of the current page
- `syntax_highlighting_enabled` - Whether syntax highlighting is enabled

For dynamic pages (files like `[slug].md`), you also get the dynamic parameter as a variable. See [Dynamic Page Paths](/blog/dynamic-paths) for details.

The `_/content.md` template has access to these same variables, plus a special `content` variable containing the rendered HTML of your page. See [Theming & CSS](/blog/theming#the-content-template) for details.

### The `pages()` Function

The `pages()` function returns **all pages** on your site, each with:

- `url` - The page URL
- `file_path` - The source file path
- Plus **all frontmatter fields** from that page

**List all pages:**

{% raw %}
```jinja
{% for page in pages() %}
- [{{ page.title }}]({{ page.url }})
{% endfor %}
```
{% endraw %}

**Filter to a section:**

{% raw %}
```jinja
{% for post in pages(within="/blog") %}
- [{{ post.title }}]({{ post.url }}) - {{ post.description }}
{% endfor %}
```
{% endraw %}

The `within` argument filters pages to a URL prefix. It automatically **excludes the index page** of that section (so `/blog/` won't appear when listing `/blog` posts).

### The `cache_bust()` Function

The `cache_bust()` function adds a content-based hash to asset URLs for cache invalidation. See [Assets & Static Files](/blog/assets#cache-busting) for details.

### Filters

Jinja provides filters to transform values. Chain them with the `|` operator:

{% raw %}
```jinja
{% for post in pages(within="/blog") | sort(attribute="order") %}
- {{ post.title }}
{% endfor %}
```
{% endraw %}

Common filters:

- `sort(attribute="field")` - Sort by a field
- `reverse` - Reverse order
- `first` / `last` - Get first or last item
- `length` - Count items
- `upper` / `lower` - Change case
- `title` - Title Case
- `trim` - Remove whitespace
- `default(value="fallback")` - Provide a default value
- `join(sep=", ")` - Join array items
- `safe` - Mark HTML as safe (won't be escaped)
- `escape` - Escape HTML characters

### Conditionals

{% raw %}
```jinja
{% if post.featured %}
**Featured:** {{ post.title }}
{% endif %}

{% if pages(within="/blog") | length > 0 %}
We have blog posts!
{% else %}
No posts yet.
{% endif %}
```
{% endraw %}

You can also use `elif` for multiple conditions:

{% raw %}
```jinja
{% if count > 10 %}
Many items
{% elif count > 0 %}
Some items
{% else %}
No items
{% endif %}
```
{% endraw %}

### Loops

{% raw %}
```jinja
{% for post in pages(within="/blog") %}
{{ loop.index }}. {{ post.title }}
{% endfor %}
```
{% endraw %}

Inside loops, you get special variables:

- `loop.index` - Current iteration (1-based)
- `loop.index0` - Current iteration (0-based)
- `loop.first` - True on first iteration
- `loop.last` - True on last iteration
- `loop.length` - Total number of items

### Raw Blocks

Need to show template syntax without it being processed? Wrap it in **raw tags** - use `{% raw %}` to start a block and `{% endraw %}` to close it. Everything between them passes through unchanged.

This is useful for documentation or code examples that contain template syntax. In fact, all the code examples in this post use raw blocks to prevent them from being processed! View the source of this file to see how it's done.

### Whitespace Control

Add a `-` to trim whitespace before or after a tag:

{% raw %}
```jinja
{%- if condition -%}
  No extra whitespace around this
{%- endif -%}
```
{% endraw %}

### Real Example

Here's how the blog index page on this site works:

{% raw %}
```markdown
---
title: Blog
---

### Posts

{% for post in pages(within="/blog") | sort(attribute="order") %}
- [{{ post.title }}]({{ post.url }}) - {{ post.description }}
{% endfor %}
```
{% endraw %}

This automatically lists all blog posts, sorted by their `order` frontmatter field, with links and descriptions. Add a new post, and it appears in the list.

### Try It!

1. Open `blog/index.md`
2. Look at the `{% raw %}{% for post in pages(within="/blog") %}{% endraw %}` loop
3. Add a custom field like `featured: true` to one of your posts
4. Try filtering with `{% raw %}{% if post.featured %}{% endraw %}`

---

Next up: [Macros](/blog/macros) - build reusable components for your pages.
