#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
NUM_POSTS=500

echo "=== Generating $NUM_POSTS blog posts for Hugo and Hugs ==="
echo

# Sample content paragraphs to make posts realistic
read -r -d '' CONTENT << 'EOF' || true
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.

Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.

Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit.

## Code Example

Here is some example code that demonstrates a common pattern:

```python
def hello_world():
    print("Hello, World!")
    return True
```

## More Content

Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur? Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse quam nihil molestiae consequatur.

At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga.

## Conclusion

Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus, omnis voluptas assumenda est, omnis dolor repellendus.
EOF

# Clean existing generated posts
echo "Cleaning existing posts..."
rm -rf "$SCRIPT_DIR/hugo-site/content/blog/post-"*
rm -rf "$SCRIPT_DIR/hugs-site/blog/post-"*

echo "Generating posts..."

for i in $(seq -w 1 $NUM_POSTS); do
    # Generate a date spread across 2023-2024
    day_offset=$((10#$i % 730))
    post_date=$(date -d "2023-01-01 + $day_offset days" +%Y-%m-%d 2>/dev/null || date -v+${day_offset}d -j -f "%Y-%m-%d" "2023-01-01" +%Y-%m-%d 2>/dev/null || echo "2024-01-$((10#$i % 28 + 1))")

    title="Blog Post Number $i"
    description="This is the description for blog post number $i in the benchmark series."

    # Hugo post (single file)
    cat > "$SCRIPT_DIR/hugo-site/content/blog/post-$i.md" << HUGO_EOF
---
title: "$title"
date: $post_date
description: "$description"
---

# $title

$CONTENT
HUGO_EOF

    # Hugs post (directory with index.md)
    mkdir -p "$SCRIPT_DIR/hugs-site/blog/post-$i"
    cat > "$SCRIPT_DIR/hugs-site/blog/post-$i/index.md" << HUGS_EOF
---
title: $title
date: $post_date
description: $description
---

# $title

$CONTENT
HUGS_EOF

    # Progress indicator
    if (( 10#$i % 50 == 0 )); then
        echo "  Generated $i/$NUM_POSTS posts..."
    fi
done

echo
echo "Done! Generated $NUM_POSTS posts in both Hugo and Hugs sites."
echo
echo "Hugo posts: $SCRIPT_DIR/hugo-site/content/blog/"
echo "Hugs posts: $SCRIPT_DIR/hugs-site/blog/"
