---
title: About
description: Learn how pages work in Hugs
---

# How pages work

This page exists because there's a file called `about.md` in your site folder. That's how Hugs works — one markdown file, one page.

The file path becomes the URL:

- `about.md` → `/about`
- `contact.md` → `/contact`
- `blog/hello.md` → `/blog/hello`

## Creating a new page

Want a new page? Create a new `.md` file.

{% call tryit() %}
1. Create a file called `hello.md` in your site folder
2. Add some content (even just `# Hello!` works)
3. Visit `/hello` in your browser
{% endcall %}

## Adding to navigation

Your new page exists, but it's not in the nav yet. Open `_/nav.md` and you'll see:

```markdown
[Home](/)
[About](/about)
[Blog](/blog)
```

These are just markdown links. The `_/nav.md` file is a shared component — it gets pulled into every page on your site, so any changes you make here show up everywhere.

{% call tryit() %}
Add your new link to `_/nav.md`:

```markdown
[Home](/)
[About](/about)
[Hello](/hello)
[Blog](/blog)
```

Save, and the nav updates across every page.
{% endcall %}

---

Ready for more? The [Blog](/blog) section covers everything else in detail.
