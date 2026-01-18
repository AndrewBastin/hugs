---
title: Templating
description: Variables and logic in your markdown
order: 4
tags:
  - templates
---

### Your markdown is a template

Every page you write can do more than show text. Hugs runs your markdown through [Jinja](https://jinja.palletsprojects.com/) (via [MiniJinja](https://docs.rs/minijinja)), so you get variables, loops, conditionals — the works.

> **Note:** MiniJinja is a Rust implementation of Jinja2. Most Jinja syntax works, but some advanced features may differ. Check the [MiniJinja docs](https://docs.rs/minijinja) if something doesn't work as expected.

### Drop in values

Double curly braces output whatever's inside:

{% raw %}
```jinja
The page title is: {{ title }}
```
{% endraw %}

### What's available

Every page comes with:

- `title` — from frontmatter
- `url` — the page's URL
- `path_class` — a CSS-friendly class based on the URL (`blog my-post`)
- `base` — base URL path for the page
- `syntax_highlighting_enabled` — whether code highlighting is on

Dynamic pages (like `[slug].md`) also get their parameter as a variable. See [Dynamic Page Paths](/blog/dynamic-paths).

The `_/content.md` template gets all these plus `content` — your rendered HTML. More in [Theming & CSS](/blog/theming#the-content-template).

### List your pages with `pages()`

The `pages()` function gives you every page on your site:

{% raw %}
```jinja
{% for page in pages() %}
- [{{ page.title }}]({{ page.url }})
{% endfor %}
```
{% endraw %}

Each page comes with `url`, `file_path`, and all its frontmatter fields.

Want just one section? Use `within`:

{% raw %}
```jinja
{% for post in pages(within="/blog") %}
- [{{ post.title }}]({{ post.url }}) - {{ post.description }}
{% endfor %}
```
{% endraw %}

This filters to that URL prefix and skips the section's index page automatically.

### More built-in functions

**`cache_bust()`** — adds a content hash to asset URLs for cache invalidation. See [Assets & Static Files](/blog/assets#cache-busting).

**`readtime()`** — estimates reading time:

{% raw %}
```jinja
{{ readtime(content) }} min read
```
{% endraw %}

It strips code, HTML, and markdown before counting. Default is 200 words/minute — change it in config:

```toml
[build]
reading_speed = 250
```

### The `datefmt` filter

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

### Transform with filters

Filters modify values. Chain them with `|`:

{% raw %}
```jinja
{% for post in pages(within="/blog") | sort(attribute="order") %}
- {{ post.title }}
{% endfor %}
```
{% endraw %}

The useful ones:

- `sort(attribute="field")` — sort by a field
- `reverse` — flip the order
- `first` / `last` — grab one item
- `length` — count items
- `upper` / `lower` — change case
- `title` — Title Case
- `trim` — strip whitespace
- `default(value="fallback")` — provide a fallback
- `join(sep=", ")` — combine array items
- `flatten` — flatten nested arrays into one
- `safe` — trust HTML (won't escape it)
- `escape` — escape HTML characters

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

Multiple branches? Use `elif`:

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

Inside loops you get:

- `loop.index` — current iteration (starts at 1)
- `loop.index0` — starts at 0
- `loop.first` — true on first pass
- `loop.last` — true on last pass
- `loop.length` — total items

### Show template code without running it

Wrap code in raw tags to pass it through unchanged. Start with `{% raw %}` and end with `{% endraw %}`.

Every code example on this page uses raw blocks — otherwise Jinja would try to process them. View the source to see how.

### Control whitespace

Add `-` to trim space around tags:

{% raw %}
```jinja
{%- if condition -%}
  No extra whitespace around this
{%- endif -%}
```
{% endraw %}

### Putting it together

Here's how the blog index lists posts:

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

Add a post, it shows up. Sorted by `order`, linked, with descriptions. Automatic.

### When you're stuck: `help`

Not sure what's available? Hugs has a built-in debugger.

See everything in scope:

{% raw %}
```jinja
{{ help() }}
```
{% endraw %}

See what filters work on a value:

{% raw %}
```jinja
{{ some_value | help }}
```
{% endraw %}

See available tests:

{% raw %}
```jinja
{% if some_value is help %}{% endif %}
```
{% endraw %}

This throws an error with all the details. Remove it when you're done.

{% call tryit() %}
1. Open `blog/index.md`
2. Look at the `{% raw %}{% for post in pages(within="/blog") %}{% endraw %}` loop
3. Add `featured: true` to one of your posts
4. Try filtering with `{% raw %}{% if post.featured %}{% endraw %}`
{% endcall %}

---
