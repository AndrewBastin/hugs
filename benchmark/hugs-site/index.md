---
title: Home
---

# Welcome to the Benchmark Blog

This is a benchmark blog used to compare static site generator build times.

## Recent Posts

{% for post in pages(within="/blog/")[:10] %}
- [{{ post.title }}]({{ post.url }}) - {{ post.date }}
{% endfor %}
