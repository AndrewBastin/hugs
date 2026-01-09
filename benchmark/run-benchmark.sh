#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HUGS_BIN="$PROJECT_ROOT/target/release/hugs"

echo "=== Hugo vs Hugs Build Time Benchmark ==="
echo

# Check if Hugs binary exists
if [ ! -f "$HUGS_BIN" ]; then
    echo "ERROR: Hugs binary not found at $HUGS_BIN"
    echo "Run ./benchmark/setup.sh first to build Hugs."
    exit 1
fi

# Check if content has been generated
HUGO_POSTS=$(find "$SCRIPT_DIR/hugo-site/content/blog" -name "post-*.md" 2>/dev/null | wc -l)
HUGS_POSTS=$(find "$SCRIPT_DIR/hugs-site/blog" -type d -name "post-*" 2>/dev/null | wc -l)

if [ "$HUGO_POSTS" -eq 0 ] || [ "$HUGS_POSTS" -eq 0 ]; then
    echo "ERROR: No blog posts found."
    echo "Run ./benchmark/generate-content.sh first to generate posts."
    exit 1
fi

echo "Hugo posts: $HUGO_POSTS"
echo "Hugs posts: $HUGS_POSTS"
echo

# Clean output directories
rm -rf "$SCRIPT_DIR/hugo-site/public"
rm -rf "$SCRIPT_DIR/hugs-site/dist"

echo "Running benchmark with hyperfine..."
echo

hyperfine \
    --warmup 10 \
    --min-runs 10 \
    --prepare "rm -rf '$SCRIPT_DIR/hugo-site/public' '$SCRIPT_DIR/hugs-site/dist'" \
    --export-markdown "$SCRIPT_DIR/results.md" \
    --command-name "Hugo" "hugo --source '$SCRIPT_DIR/hugo-site' --quiet" \
    --command-name "Hugs" "'$HUGS_BIN' build '$SCRIPT_DIR/hugs-site' --output '$SCRIPT_DIR/hugs-site/dist'"

echo
echo "Results saved to: $SCRIPT_DIR/results.md"
