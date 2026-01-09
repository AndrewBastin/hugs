---
title: RSS & Atom Feeds
description: Syndicate your content with RSS and Atom feeds
order: 10
---

Feeds let people subscribe to your site and get notified when you publish new content. Hugs supports both **RSS 2.0** and **Atom** formats - the two most widely used feed standards.

### Why Feeds?

RSS and Atom feeds power:
- **Feed readers** - Apps like Feedly, NetNewsWire, or Thunderbird
- **Podcatchers** - If your site hosts a podcast
- **Aggregators** - Sites that collect posts from multiple sources
- **Automation** - Services like IFTTT, Zapier, or custom scripts

Adding feeds to your site is a few lines in `config.toml`.

### Basic Feed Setup

Add a `[[feeds]]` section to your `config.toml`:

```toml
[[feeds]]
name = "blog"
source = "/blog"
output_rss = "feed.xml"
```

This creates `feed.xml` in your build output, containing all pages under `/blog/`.

The double brackets `[[feeds]]` mean this is an array - you can define multiple feeds for different sections of your site.

### RSS vs Atom

Both formats accomplish the same thing. Choose based on your audience:

- **RSS** - More widely recognized name, simpler format
- **Atom** - More rigorous specification, slightly better for complex content

You can generate **both** from the same configuration:

```toml
[[feeds]]
name = "blog"
source = "/blog"
output_rss = "feed.xml"
output_atom = "atom.xml"
```

This creates two files with the same content in different formats. Feed readers typically support both.

### Feed Configuration Options

Here's a feed with all available options:

```toml
[[feeds]]
name = "blog"                          # Identifier (required)
source = "/blog"                       # Path filter (required)
output_rss = "feed.xml"                # RSS filename (optional)
output_atom = "atom.xml"               # Atom filename (optional)
title = "My Blog Feed"                 # Feed title (optional)
description = "Latest posts from..."   # Feed description (optional)
limit = 20                             # Max items (default: 20)
```

**Required fields:**
- `name` - A unique identifier for the feed
- `source` - The URL path to include pages from

**Output files** (at least one required):
- `output_rss` - Filename for RSS 2.0 output
- `output_atom` - Filename for Atom output

**Optional overrides:**
- `title` - Defaults to your site title
- `description` - Defaults to your site description
- `limit` - How many items to include (default: 20)

### How Source Filtering Works

The `source` field determines which pages appear in the feed:

```toml
source = "/blog"
```

This includes:
- `/blog/my-first-post`
- `/blog/another-post`
- `/blog/2024/january-update`

But **excludes**:
- `/blog/` (the index page itself)
- `/about`
- `/projects/something`

Use different sources for different feeds:

```toml
[[feeds]]
name = "blog"
source = "/blog"
output_rss = "blog.xml"

[[feeds]]
name = "tutorials"
source = "/tutorials"
output_rss = "tutorials.xml"

[[feeds]]
name = "everything"
source = "/"
output_rss = "all.xml"
limit = 50
```

### Adding Dates to Posts

For feeds to work well, your posts need **dates**. Add a date field to your frontmatter:

```markdown
---
title: My Post
description: A great post
date: 2024-06-15
---
```

Hugs recognizes several date field names:
- `date`
- `published`
- `created`
- `pubDate`

And several formats:

```yaml
# Simple date
date: 2024-06-15

# With time (ISO 8601)
date: 2024-06-15T10:30:00Z

# With time (space-separated)
date: 2024-06-15 10:30:00
```

Posts are sorted by date in the feed, with most recent first. Posts without dates appear at the end.

### Feed Content from Frontmatter

Each feed item pulls content from your page's frontmatter:

- **Title** - From `title` field (falls back to "Untitled")
- **Link** - The page's URL
- **Date** - From `date`, `published`, `created`, or `pubDate` field
- **Description** - From `description`, `summary`, or `excerpt` field
- **Author** - From `author` field (falls back to site author)

A well-structured blog post for feeds:

```markdown
---
title: Getting Started with Hugs
description: Learn how to build your first static site with Hugs
date: 2024-06-15
author: Jane Doe
---

Your content here...
```

### Required Site Configuration

Feeds require your site URL to be set. Without it, Hugs can't generate valid links:

```toml
[site]
title = "My Site"
url = "https://mysite.com"
```

If you try to build feeds without a URL, you'll see a helpful error message.

### Multiple Feeds Example

A typical site might have:

```toml
[site]
title = "My Site"
description = "Thoughts on code and life"
url = "https://mysite.com"
author = "Your Name"

# Main blog feed
[[feeds]]
name = "blog"
source = "/blog"
output_rss = "feed.xml"
output_atom = "atom.xml"

# Separate feed for tutorials
[[feeds]]
name = "tutorials"
source = "/tutorials"
output_rss = "tutorials.xml"
title = "Tutorial Feed"
description = "Step-by-step coding tutorials"
limit = 10
```

### Linking to Your Feeds

Let visitors know your feeds exist. Add links in your header, footer, or nav:

```markdown
[RSS](/feed.xml) | [Atom](/atom.xml)
```

Or use the standard `<link>` tag in your HTML (if you're customizing templates):

```html
<link rel="alternate" type="application/rss+xml" title="RSS" href="/feed.xml">
<link rel="alternate" type="application/atom+xml" title="Atom" href="/atom.xml">
```

### Validating Your Feeds

After building, check that your feeds are valid:

1. **Build your site**: `hugs build .`
2. **Open the feed file**: Look at `dist/feed.xml`
3. **Validate online**: Use the [W3C Feed Validation Service](https://validator.w3.org/feed/)

Common issues:
- **Missing URL** - Set `url` in `[site]` config
- **Missing dates** - Add `date` to your posts' frontmatter
- **Empty feed** - Check that `source` matches your page URLs

### Feed Readers for Testing

Try subscribing to your own feed to see how it looks:

- **Feedly** - Popular web-based reader
- **NetNewsWire** - Free, native Mac/iOS app
- **Thunderbird** - Email client with built-in feed support
- **Inoreader** - Web-based with good free tier

Paste your feed URL (e.g., `https://mysite.com/feed.xml`) into any reader to test.

### Tips

**Keep descriptions concise** - Feed readers show limited text. 1-2 sentences work best.

**Use consistent dates** - Pick a date format and stick with it across all posts.

**Don't over-limit** - The default of 20 items is usually good. Going lower might miss content for infrequent visitors.

**Update your sitemap** - Feed URLs aren't automatically added to the sitemap, but search engines find them through your `<link>` tags.

### Try It!

1. Add a feed to your `config.toml`:
   ```toml
   [[feeds]]
   name = "blog"
   source = "/blog"
   output_rss = "feed.xml"
   ```

2. Make sure your site has a URL set:
   ```toml
   [site]
   url = "https://example.com"
   ```

3. Add dates to your blog posts if you haven't already

4. Run `hugs build .` and check `dist/feed.xml`

5. Paste the XML into the [W3C validator](https://validator.w3.org/feed/) to verify

---

Next up: [Building This Site](/blog/building-this-site) - a meta look at how this tutorial site is structured.
