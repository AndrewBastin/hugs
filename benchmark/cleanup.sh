#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Cleaning up benchmark files ==="

# Remove generated posts
rm -rf "$SCRIPT_DIR/hugo-site/content/blog/post-"*
rm -rf "$SCRIPT_DIR/hugs-site/blog/post-"*

# Remove build outputs
rm -rf "$SCRIPT_DIR/hugo-site/public"
rm -rf "$SCRIPT_DIR/hugs-site/dist"

# Remove benchmark results
rm -f "$SCRIPT_DIR/results.md"

echo "Done!"
