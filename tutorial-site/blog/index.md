---
title: Blog
description: Learn Hugs features through tutorial posts
---

# Explore blogs

A portfolio or a personal website is somewhat incomplete without a blog. 
This is where you can share your thoughts, experiences, and knowledge with the world. 

You're in the `blog/` folder. This file — `index.md` — is what shows up when you visit `/blog`. 
Every other markdown file here becomes a post: `my-post.md` turns into `/blog/my-post`.

Work through the tutorial posts on everything you need to know about **Hugs** in order, or pick what you need. Each one builds on the last, but they stand alone too.


## Tutorial Posts

{% for post in pages(within="/blog") | sort(attribute="order") %}
{% if post.order %}- [{{ post.title }}]({{ post.url }}) — {{ post.description }}
{% endif %}{% endfor %}


## Browse by topic

[basics](/blog/basics) · [templates](/blog/templates) · [styling](/blog/styling) · [publishing](/blog/publishing)


Start with [Config File](/blog/config) — it's where everything begins.
