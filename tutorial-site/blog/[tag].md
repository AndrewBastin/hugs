---
title: "{{ tag | title }}"
description: "Posts tagged with {{ tag }}"
tag:
  - basics
  - templates
  - styling
  - publishing
---

### Posts tagged "{{ tag }}"

{% for post in pages(within="/blog") | sort(attribute="order") %}
{% if post.tags %}{% for t in post.tags %}{% if t == tag %}
- [{{ post.title }}]({{ post.url }}) — {{ post.description }}
{% endif %}{% endfor %}{% endif %}
{% endfor %}

---

[← Back to all posts](/blog)
