---
title: Assets & Static Files
description: Images, CSS, JavaScript, and other files
order: 8
tags:
  - styling
---

### Everything that isn't markdown

Images, fonts, PDFs, JavaScript — anything that's not a `.md` file gets copied to your built site as-is.

### Images

Drop them anywhere (except `_/`):

```
my-site/
├── images/
│   ├── logo.png
│   └── hero.jpg
├── index.md
└── about.md
```

Reference with standard markdown:

```markdown
![My Logo](/images/logo.png)
```

Paths start with `/` — they're relative to your site root.

### Organize however you want

```
my-site/
├── images/           → /images/
├── fonts/            → /fonts/
├── downloads/        → /downloads/
│   └── resume.pdf    → /downloads/resume.pdf
└── scripts/          → /scripts/
```

Folder structure = URL structure.

### Favicon

Put `favicon.ico` in the root. Browsers request it automatically.

### JavaScript

HTML works in markdown, so scripts work too:

```html
<script src="/scripts/my-script.js"></script>
```

Or inline:

```html
<script>
  console.log('Hello!');
</script>
```

### Cache busting

The `cache_bust()` function adds a content hash to URLs:

{% raw %}
```html
<link rel="stylesheet" href="{{ cache_bust(path='/styles/custom.css') }}">
```
{% endraw %}

Outputs `/styles/custom.a1b2c3f4.css`. File changes → hash changes → browsers fetch fresh.

Built-in `theme.css` and `highlight.css` use this automatically.

### What gets copied

During build, everything copies except:
- `.md` files (become HTML)
- `_/` folder (structural files)
- `config.toml` (not public)

### Social images

The `image` field in frontmatter is used for social media previews:

```markdown
---
title: My Post
image: /images/post-cover.jpg
---
```

More on this in [SEO & Meta Tags](/blog/seo).

{% call tryit() %}
1. Create an `images/` folder
2. Add any image file
3. Reference it: `![My image](/images/filename.jpg)`
{% endcall %}

---