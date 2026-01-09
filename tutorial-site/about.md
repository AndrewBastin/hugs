---
title: About
description: Learn how pages work in Hugs
---

## How Pages Work

I'm `about.md`, and I exist because someone created a file called `about.md` in the root folder. That's it - that's how pages work in Hugs!

**One markdown file = one page.** The file path becomes the URL:

- `about.md` → `/about`
- `contact.md` → `/contact`
- `blog/hello.md` → `/blog/hello`

---

### Creating a New Page

Want to add a page? Create a new `.md` file. Try it now:

1. Create a file called `hello.md` in your site folder
2. Add some content (even just `# Hello!` works)
3. Visit `/hello` in your browser

That's a new page! Easy, right ?

---

### Adding to Navigation

Your new page exists, but it's not in the nav yet. Open `_/nav.md` and you'll see:

```markdown
[Home](/)
[About](/about)
[Blog](/blog)
```

Just add another link:

```markdown
[Home](/)
[About](/about)
[Hello](/hello)
[Blog](/blog)
```

Save, and your nav updates on every page.

---

There is more to talk about! Head to the [Blog](/blog) section for more things in detail!
