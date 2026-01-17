---
heading: "Try it yourself"
---

{#
  This is a macro — a reusable component you can drop into any page.

  Macros live in `_/macros/`. The filename becomes the macro name, so this
  file (tryit.md) creates a macro called `tryit`. Frontmatter fields become
  parameters with default values. The special `caller()` function outputs
  whatever content you pass between the call tags.

  I use this one to create "Try it yourself" exercise boxes in the tutorials.
  The styling lives in theme.css under TRYIT BOXES.

  Usage:

    {% call tryit() %}
    1. Do this
    2. Then do this
    {% endcall %}

  You can also override the heading:

    {% call tryit(heading="Exercise") %}
    Your content here (markdown works!)
    {% endcall %}
#}

<div class="tryit">

**{{ heading }}**

{{ caller() }}

</div>
