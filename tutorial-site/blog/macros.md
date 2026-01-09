---
title: Macros
description: Build reusable components for your pages
order: 4.5
---

Macros let you create **reusable template components**. Define them once, use them everywhere - perfect for cards, callouts, buttons, or any repeated pattern on your site.

### Creating a Macro

Macros live in the `_/macros/` directory. Each `.md` file becomes a macro:

{% raw %}
```
_/
├── macros/
│   ├── note.md      → {% call note() %}...{% endcall %}
│   ├── card.md      → {% call card() %}...{% endcall %}
│   └── button.md    → {% call button() %}...{% endcall %}
├── header.md
├── footer.md
└── nav.md
```
{% endraw %}

The filename becomes the macro name - `note.md` creates a macro called `note`.

### Basic Example

Here's a simple callout macro. Create `_/macros/note.md`:

{% raw %}
```jinja
---
type: "info"
---
> **{{ type | upper }}**: {{ caller() }}
```
{% endraw %}

That's it! The **frontmatter** defines parameters with default values, and **`caller()`** is where the content you pass in will appear.

### Using Macros

Use the `{% raw %}{% call %}{% endraw %}` syntax to invoke a macro:

{% raw %}
```jinja
{% call note() %}
Remember to save your work frequently!
{% endcall %}
```
{% endraw %}

This outputs a blockquote with "INFO" as the default type. Pass a different value to customize it:

{% raw %}
```jinja
{% call note(type="warning") %}
This action cannot be undone.
{% endcall %}
```
{% endraw %}

### Markdown Works Everywhere

Both the **macro body** and the **content you pass in** support full markdown. Macros are processed during template rendering, which happens *before* markdown conversion.

Your macro can output markdown:

{% raw %}
```markdown
---
title: ""
---
### {{ title }}

{{ caller() }}

---
```
{% endraw %}

And the content you pass in can use markdown too:

{% raw %}
```jinja
{% call section(title="Features") %}
- **Fast** builds
- **Simple** syntax
- **Flexible** templates
{% endcall %}
```
{% endraw %}

### Parameters

Frontmatter values become parameters with defaults:

{% raw %}
```markdown
---
type: "primary"
size: "medium"
disabled: false
---
<button class="btn btn--{{ type }} btn--{{ size }}"{% if disabled %} disabled{% endif %}>
  {{ caller() }}
</button>
```
{% endraw %}

When calling the macro:
- Omit a parameter to use its default
- Pass a value to override the default

{% raw %}
```jinja
{% call button() %}Click me{% endcall %}
{% call button(type="danger", size="large") %}Delete{% endcall %}
{% call button(disabled=true) %}Unavailable{% endcall %}
```
{% endraw %}

### Accessing Page Variables

Macros can access variables from your page. If your page frontmatter has:

```yaml
---
title: My Post
author: Jane
---
```

Your macro can use those variables:

{% raw %}
```markdown
---
---
Written by **{{ author }}** for *{{ title }}*

{{ caller() }}
```
{% endraw %}

### Practical Example: Card Component

Here's a card macro that combines several features. Create `_/macros/card.md`:

{% raw %}
```markdown
---
title: ""
variant: "default"
---
<div class="card card--{{ variant }}">
{% if title %}<h3>{{ title }}</h3>{% endif %}

{{ caller() }}

</div>
```
{% endraw %}

Use it throughout your site:

{% raw %}
```jinja
{% call card(title="Getting Started", variant="featured") %}
Welcome to the documentation! This guide will help you
get up and running in minutes.

[Read more →](/docs/getting-started)
{% endcall %}
```
{% endraw %}

### Try It!

1. Create `_/macros/` directory in your site
2. Add a simple macro like the `note.md` example above
3. Use `{% raw %}{% call note() %}Your message{% endcall %}{% endraw %}` in any page
4. Experiment with parameters and markdown content

---

Next up: [Syntax Highlighting](/blog/syntax-highlighting) - make your code blocks look great.
