#!/usr/bin/env bash
set -e

echo "=== Hugs vs Hugo Benchmark Setup ==="
echo

# Check for Hugo
if command -v hugo &> /dev/null; then
    echo "Hugo found: $(hugo version)"
else
    echo "ERROR: Hugo is not installed."
    echo "Install it from: https://gohugo.io/installation/"
    exit 1
fi

# Check for hyperfine
if command -v hyperfine &> /dev/null; then
    echo "hyperfine found: $(hyperfine --version)"
else
    echo "ERROR: hyperfine is not installed."
    echo "Install it from: https://github.com/sharkdp/hyperfine"
    exit 1
fi

# Build Hugs in release mode
echo
echo "Building Hugs in release mode..."
cd "$(dirname "$0")/.."
cargo build --release

echo
echo "Setup complete!"
echo "Hugs binary: $(pwd)/target/release/hugs"
echo
echo "Next steps:"
echo "  1. Run ./benchmark/generate-content.sh to generate blog posts"
echo "  2. Run ./benchmark/run-benchmark.sh to run the benchmark"
