---
title: Templating
description: Variables and logic in your markdown
order: 4
---

Your markdown files aren't just static text - they're **templates**. Hugs uses [Tera](https://keats.github.io/tera/), a powerful templating engine with Jinja2-style syntax.

### Variables

Use double curly braces to output values:

{% raw %}
```markdown
The page title is: {{ title }}
```
{% endraw %}

### Available Variables

Every page has access to these built-in variables:

- **`title`** - The page title from frontmatter
- **`path_class`** - A CSS-friendly class based on the URL (e.g., `blog-my-post`)
- **`base`** - The base URL path

For dynamic pages (files like `[slug].md`), you also get the dynamic parameter as a variable. See [Dynamic Page Paths](/blog/dynamic-paths) for details.

### The `pages()` Function

This is where things get powerful. The `pages()` function returns **all pages** on your site, each with:

- `url` - The page URL
- `file_path` - The source file path
- Plus **all frontmatter fields** from that page

**List all pages:**

{% raw %}
```markdown
{% for page in pages() %}
- [{{ page.title }}]({{ page.url }})
{% endfor %}
```
{% endraw %}

**Filter to a section:**

{% raw %}
```markdown
{% for post in pages(within="/blog") %}
- [{{ post.title }}]({{ post.url }}) - {{ post.description }}
{% endfor %}
```
{% endraw %}

The `within` argument filters pages to a URL prefix. It automatically **excludes the index page** of that section (so `/blog/` won't appear when listing `/blog` posts).

### Filters

Tera provides filters to transform values. Chain them with the `|` operator:

{% raw %}
```markdown
{% for post in pages(within="/blog") | sort(attribute="order") %}
- {{ post.title }}
{% endfor %}
```
{% endraw %}

Common filters:
- `sort(attribute="field")` - Sort by a frontmatter field
- `reverse` - Reverse order
- `first` / `last` - Get first or last item
- `length` - Count items
- `upper` / `lower` - Change case
- `default(value="fallback")` - Provide a default

### Conditionals

{% raw %}
```markdown
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

### Loops

{% raw %}
```markdown
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

### Raw Blocks

Need to show Tera syntax without it being processed? Wrap it in **raw tags** - use the tag name `raw` to start a block and `endraw` to close it. Everything between them passes through unchanged.

This is useful for documentation or code examples that contain template syntax. In fact, all the code examples in this post use raw blocks to prevent them from being processed! View the source of this file to see how it's done.

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

### Learn More

This post covers the most common templating features, but Tera has much more. Check out the [Tera documentation](https://keats.github.io/tera/docs/#templates) for the full reference.

**Note:** Some features of Tera may not work in Hugs.

---

Next up: [Syntax Highlighting](/blog/syntax-highlighting) - make your code blocks look great.
