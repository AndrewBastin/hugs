{#
  I use `{{ content }}` to output the page's actual content, but I've added
  some extras for blog posts: automatic title rendering, clickable tag badges,
  and prev/next navigation cards at the bottom.

  The navigation cards use the `order` field from each post's frontmatter
  to figure out which posts come before and after. The styling for all of
  this lives in theme.css.
#}

{% if path_class is startingwith("blog ") %}
# {{ title }}
{% endif %}

{{ content }}

{% if path_class is startingwith("blog ") %}
{% if order is defined %}
{% set all_posts = pages(within="/blog") | selectattr("order") | list %}
{% set prev_post = all_posts | selectattr("order", "eq", order - 1) | first %}
{% set next_post = all_posts | selectattr("order", "eq", order + 1) | first %}

{% if tags is defined %}
<div class="tag-list">
{% for tag in tags %}<a href="/blog/{{ tag }}" class="tag-badge">{{ tag }}</a>{% endfor %}
</div>
{% endif %}

<div class="nav-cards">
{% if prev_post %}
<a href="{{ prev_post.url }}" class="nav-card">
<span class="nav-card-arrow">‹</span>
<div>
<div class="nav-card-title">{{ prev_post.title }}</div>
<div class="nav-card-desc">{{ prev_post.description }}</div>
</div>
</a>
{% else %}
<div class="nav-card-spacer"></div>
{% endif %}
{% if next_post %}
<a href="{{ next_post.url }}" class="nav-card nav-card-next">
<div>
<div class="nav-card-title">{{ next_post.title }}</div>
<div class="nav-card-desc">{{ next_post.description }}</div>
</div>
<span class="nav-card-arrow">›</span>
</a>
{% else %}
<div class="nav-card-spacer"></div>
{% endif %}
</div>
{% endif %}

{% endif%}
