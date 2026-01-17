---
title: Macros
description: Build reusable components for your pages
order: 5
tags:
  - templates
---

### Write once, use everywhere

Find yourself copying the same card layout or callout box across pages? That's what macros fix. Define a component once, then drop it anywhere.

Macros live in `_/macros/`. The filename becomes the macro name:

{% raw %}
```
_/macros/
├── note.md      → {% call note() %}...{% endcall %}
├── card.md      → {% call card() %}...{% endcall %}
└── button.md    → {% call button() %}...{% endcall %}
```
{% endraw %}

### A simple example

Create `_/macros/note.md`:

{% raw %}
```jinja
---
type: "info"
---
> **{{ type | upper }}**: {{ caller() }}
```
{% endraw %}

Frontmatter defines parameters (with defaults). `caller()` is where your content goes.

Now use it:

{% raw %}
```jinja
{% call note() %}
Remember to save your work frequently!
{% endcall %}
```
{% endraw %}

Override the default:

{% raw %}
```jinja
{% call note(type="warning") %}
This action cannot be undone.
{% endcall %}
```
{% endraw %}

### Markdown works inside macros

Both the macro body and the content you pass in support full markdown — macros run before markdown conversion.

{% raw %}
```jinja
{% call section(title="Features") %}
- **Fast** builds
- **Simple** syntax
- **Flexible** templates
{% endcall %}
```
{% endraw %}

### Parameters with defaults

Every frontmatter field becomes a parameter. The value you set is the default — used when the caller doesn't specify one.

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

This macro has three parameters: `type` defaults to `"primary"`, `size` to `"medium"`, `disabled` to `false`.

When you call it, you can override any of them:

{% raw %}
```jinja
{% call button() %}Click me{% endcall %}
<!-- uses all defaults: type="primary", size="medium", disabled=false -->

{% call button(type="danger", size="large") %}Delete{% endcall %}
<!-- overrides type and size, disabled stays false -->

{% call button(disabled=true) %}Unavailable{% endcall %}
<!-- only overrides disabled -->
```
{% endraw %}

### Page variables are available

Macros can access the page's frontmatter. If your page has `author: Jane`, your macro can use `{{ author }}`.

### Putting it together: a card component

`_/macros/card.md`:

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

Use it:

{% raw %}
```jinja
{% call card(title="Getting Started", variant="featured") %}
Welcome! This guide gets you up and running in minutes.

[Read more →](/docs/getting-started)
{% endcall %}
```
{% endraw %}

{% call tryit() %}
1. Create `_/macros/` directory in your site
2. Add the `note.md` example above
3. Use `{% raw %}{% call note() %}Your message{% endcall %}{% endraw %}` in any page
{% endcall %}

---
