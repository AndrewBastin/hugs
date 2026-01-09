# Hugo vs Hugs Benchmark

This benchmark compares build times between [Hugo](https://gohugo.io/) and Hugs using [hyperfine](https://github.com/sharkdp/hyperfine).

## Prerequisites

- **Hugo**: Install from https://gohugo.io/installation/
- **hyperfine**: Install from https://github.com/sharkdp/hyperfine
- **Rust toolchain**: Required to build Hugs

## Quick Start

```bash
# Run everything (build, generate, benchmark, cleanup)
./benchmark/benchmark.sh
```

## Individual Scripts

```bash
./benchmark/setup.sh           # Check deps & build Hugs
./benchmark/generate-content.sh # Generate 500 posts
./benchmark/run-benchmark.sh    # Run hyperfine comparison
./benchmark/cleanup.sh          # Remove generated files
```

## What's Being Compared

Both sites have identical content:
- 500 blog posts with ~500 words each
- Markdown with YAML frontmatter
- Code blocks and headings
- Similar minimal templates/themes

The benchmark measures **cold build time** - building from source to static HTML with no caching.

## Results

After running `run-benchmark.sh`, results are saved to `results.md` in this directory.

## Directory Structure

```
benchmark/
├── benchmark.sh        # All-in-one script
├── setup.sh            # Check dependencies & build
├── generate-content.sh # Generate blog posts
├── run-benchmark.sh    # Run hyperfine benchmark
├── cleanup.sh          # Remove generated files
├── hugo-site/          # Hugo demo blog
└── hugs-site/          # Hugs demo blog
```
