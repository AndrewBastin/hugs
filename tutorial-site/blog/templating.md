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

### The `readtime()` Function

The `readtime()` function calculates estimated reading time in minutes for a given text:

{% raw %}
```jinja
{{ readtime(content) }} min read
```
{% endraw %}

It strips code blocks, HTML tags, and markdown syntax before counting words. The default reading speed is 200 words per minute, but you can configure this in your `config.toml`:

```toml
[build]
reading_speed = 250  # words per minute
```

### The `datefmt` Filter

The `datefmt` filter formats dates using strftime patterns with locale support:

{% raw %}
```jinja
{{ page.date | datefmt("%B %d, %Y") }}
{# Output: "January 15, 2024" #}
```
{% endraw %}

It accepts dates in these formats:
- `2024-01-15` (YYYY-MM-DD)
- `2024-01-15T10:30:00Z` (ISO 8601)
- `2024-01-15 10:30:00` (YYYY-MM-DD HH:MM:SS)

**Locale support:**

By default, `datefmt` uses your site's `language` setting from `config.toml`. You can override it per-filter:

{% raw %}
```jinja
{{ page.date | datefmt("%A, %d %B %Y", locale="fr_FR") }}
{# Output: "lundi, 15 janvier 2024" #}

{{ page.date | datefmt("%d %B %Y", locale="de_DE") }}
{# Output: "15 Januar 2024" #}
```
{% endraw %}

**Common strftime patterns:**

| Pattern | Description | Example |
|---------|-------------|---------|
| `%Y` | Full year | 2024 |
| `%y` | Short year (2 digits) | 24 |
| `%m` | Month with leading zero | 01-12 |
| `%-m` | Month without leading zero | 1-12 |
| `%d` | Day with leading zero | 01-31 |
| `%-d` | Day without leading zero | 1-31 |
| `%e` | Day space-padded | &nbsp;1-31 |
| `%B` | Full month name | January |
| `%b` | Abbreviated month | Jan |
| `%A` | Full weekday | Monday |
| `%a` | Abbreviated weekday | Mon |
| `%H` | Hour 24h with leading zero | 00-23 |
| `%-H` | Hour 24h without leading zero | 0-23 |
| `%I` | Hour 12h with leading zero | 01-12 |
| `%-I` | Hour 12h without leading zero | 1-12 |
| `%M` | Minute | 00-59 |
| `%S` | Second | 00-59 |
| `%p` | AM/PM | AM |

**Available locales:**

Hugs supports 400+ locales via [pure-rust-locales](https://docs.rs/pure-rust-locales). Common examples:
- `en_US`, `en_GB`, `en_AU` - English variants
- `fr_FR`, `de_DE`, `es_ES`, `it_IT` - European languages
- `ja_JP`, `zh_CN`, `ko_KR` - Asian languages
- `pt_BR`, `ru_RU`, `ar_EG` - Other languages

Use underscore (`en_US`) or hyphen (`en-US`) format. See the full list at [glibc locales](https://sourceware.org/git/?p=glibc.git;a=tree;f=localedata/locales).

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

### Debugging with `help`

Not sure what variables or filters are available? Hugs provides a built-in `help` that shows you everything in your current context.

**See all available variables, functions, filters, and tests:**

{% raw %}
```jinja
{{ help() }}
```
{% endraw %}

**See what filters can be applied to a value:**

{% raw %}
```jinja
{{ some_value | help }}
```
{% endraw %}

**See what tests can be used with a value:**

{% raw %}
```jinja
{% if some_value is help %}{% endif %}
```
{% endraw %}

When you use `help`, the page will show an error with detailed information about what's available. It's a debugging tool - remove it once you've found what you need!

### Try It!

1. Open `blog/index.md`
2. Look at the `{% raw %}{% for post in pages(within="/blog") %}{% endraw %}` loop
3. Add a custom field like `featured: true` to one of your posts
4. Try filtering with `{% raw %}{% if post.featured %}{% endraw %}`

---

Next up: [Macros](/blog/macros) - build reusable components for your pages.
