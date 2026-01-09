---
title: Blog
description: Learn Hugs features through tutorial posts
---

## Welcome to the Blog

I'm `blog/index.md`, and I live in the `blog/` folder. Any markdown file you put next to me becomes a blog post - `my-post.md` becomes `/blog/my-post`. The list below updates automatically when you add new posts!

These posts are a tutorial series. Each one teaches a Hugs feature in depth. Read them in order, or jump to whatever interests you.

---

### Tutorial Posts

{% for post in pages(within="/blog") | sort(attribute="order") %}
- [{{ post.title }}]({{ post.url }}) - {{ post.description }}
{% endfor %}

---

Ready to dive in? Start with [Config File](/blog/config) to learn about site settings.
