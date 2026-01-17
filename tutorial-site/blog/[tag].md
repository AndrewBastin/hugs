---
title: "{{ tag | title }}"
description: "Posts tagged with {{ tag }}"
tag: "{{ pages(within='/blog') | map(attribute='tags') | flatten | unique | sort }}"
---

### Posts tagged "{{ tag }}"

{% for post in pages(within="/blog") | sort(attribute="order") %}
{% if post.tags %}{% for t in post.tags %}{% if t == tag %}
- [{{ post.title }}]({{ post.url }}) — {{ post.description }}
{% endif %}{% endfor %}{% endif %}
{% endfor %}

---

[← Back to all posts](/blog)
