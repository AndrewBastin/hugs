---
title: Syntax Highlighting
description: Pretty code blocks
order: 5
---

Code blocks in Hugs automatically get syntax highlighting. Just specify the language, and your code looks great.

### Basic Usage

Add a language identifier after the opening triple backticks:

~~~markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
~~~

This renders as:

```rust
fn main() {
    println!("Hello, world!");
}
```

### Supported Languages

Hugs uses [giallo](https://github.com/getzola/giallo) for syntax highlighting, which supports most popular languages including:

- **Web**: `html`, `css`, `javascript`, `typescript`, `jsx`, `tsx`, `json`
- **Backend**: `rust`, `go`, `python`, `ruby`, `java`, `c`, `cpp`
- **Shell**: `bash`, `shell`, `zsh`, `fish`
- **Config**: `toml`, `yaml`, `xml`, `ini`
- **Markup**: `markdown`, `latex`
- **And many more**: `sql`, `graphql`, `dockerfile`, `make`, `lua`, `swift`, `kotlin`...

If a language isn't recognized, the code block displays without highlighting.

### Configuration

Syntax highlighting is enabled by default. Configure it in `config.toml`:

```toml
[build.syntax_highlighting]
enabled = true
theme = "one-dark-pro"
```

### Available Themes

Hugs includes 60+ themes. Here are some popular choices:

**Dark themes:**
- `one-dark-pro` (default)
- `dracula`
- `github-dark`
- `tokyo-night`
- `catppuccin-mocha`
- `nord`
- `gruvbox-dark-medium`
- `rose-pine`

**Light themes:**
- `one-light`
- `github-light`
- `catppuccin-latte`
- `solarized-light`
- `gruvbox-light-medium`
- `rose-pine-dawn`

For the full list of 60+ themes, see the [giallo theme gallery](https://github.com/getzola/giallo).

### How It Works

When you build your site, Hugs:

1. Parses code blocks with language tags
2. Applies syntax highlighting using tree-sitter grammars
3. Generates CSS for your chosen theme at `/highlight.css`
4. The CSS is automatically included in your pages

You don't need to do anything special - it just works.

### Disabling Highlighting

If you prefer unstyled code blocks, disable highlighting:

```toml
[build.syntax_highlighting]
enabled = false
```

### Try It!

1. Open `config.toml` in your site
2. Add a `[build.syntax_highlighting]` section
3. Try `theme = "dracula"` or `theme = "github-dark"`
4. Refresh your browser to see the new colors

---

Next up: [Theming & CSS](/blog/theming) - customize your site's look and feel.
