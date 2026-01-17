---
title: Syntax Highlighting
description: Pretty code blocks
order: 6
tags:
  - styling
---

### Code that looks good

Add a language after the triple backticks, and Hugs handles the rest:

~~~markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
~~~

Renders as:

```rust
fn main() {
    println!("Hello, world!");
}
```

### Languages

Hugs uses [giallo](https://github.com/getzola/giallo) and supports most languages you'd expect:

- **Web**: `html`, `css`, `javascript`, `typescript`, `jsx`, `tsx`, `json`
- **Backend**: `rust`, `go`, `python`, `ruby`, `java`, `c`, `cpp`
- **Shell**: `bash`, `shell`, `zsh`, `fish`
- **Config**: `toml`, `yaml`, `xml`, `ini`
- **And more**: `sql`, `graphql`, `dockerfile`, `make`, `lua`, `swift`, `kotlin`...

Unrecognized languages display without highlighting.

### Pick a theme

Highlighting is on by default. Change the theme in `config.toml`:

```toml
[build.syntax_highlighting]
enabled = true
theme = "one-dark-pro"
```

**Dark themes:** `one-dark-pro` (default), `dracula`, `github-dark`, `tokyo-night`, `catppuccin-mocha`, `nord`, `gruvbox-dark-medium`, `rose-pine`

**Light themes:** `one-light`, `github-light`, `catppuccin-latte`, `solarized-light`, `gruvbox-light-medium`, `rose-pine-dawn`

60+ themes available — see the [giallo theme gallery](https://github.com/getzola/giallo) for the full list.

### Under the hood

Hugs generates `/highlight.css` with your theme's colors and includes it automatically. Nothing to configure.

To disable highlighting entirely:

```toml
[build.syntax_highlighting]
enabled = false
```

{% call tryit() %}
1. Open `config.toml`
2. Set `theme = "dracula"` under `[build.syntax_highlighting]`
3. Refresh — new colors
{% endcall %}

---