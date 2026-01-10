use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};

use crate::console;
use crate::config::{FeedConfig, SiteMetadata};
use crate::error::{HugsError, Result};
use crate::run::PageInfo;

/// Represents a page ready for feed inclusion
pub struct FeedItem {
    pub title: String,
    pub url: String,
    pub date: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub author: Option<String>,
}

/// Extract feed items from pages matching the source filter
pub fn collect_feed_items(
    pages: &[PageInfo],
    feed_config: &FeedConfig,
    site_metadata: &SiteMetadata,
) -> Vec<FeedItem> {
    let base_url = site_metadata.url.as_deref().unwrap_or("");

    let mut items: Vec<FeedItem> = pages
        .iter()
        .filter(|page| matches_source(&page.url, &feed_config.source))
        .filter_map(|page| page_to_feed_item(page, base_url, site_metadata))
        .collect();

    // Sort by date descending (most recent first)
    items.sort_by(|a, b| match (&b.date, &a.date) {
        (Some(b_date), Some(a_date)) => b_date.cmp(a_date),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    // Apply limit
    items.truncate(feed_config.limit);

    items
}

/// Check if a page URL matches the feed source filter
fn matches_source(page_url: &str, source: &str) -> bool {
    let index_url = if source.ends_with('/') {
        source.to_string()
    } else {
        format!("{}/", source)
    };

    page_url.starts_with(source) && page_url != index_url
}

/// Convert a PageInfo to a FeedItem
fn page_to_feed_item(
    page: &PageInfo,
    base_url: &str,
    site_metadata: &SiteMetadata,
) -> Option<FeedItem> {
    let title = page
        .frontmatter
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let full_url = format!("{}{}", base_url.trim_end_matches('/'), &page.url);

    let date = extract_date_from_frontmatter(&page.frontmatter);

    let summary = page
        .frontmatter
        .get("description")
        .or_else(|| page.frontmatter.get("summary"))
        .or_else(|| page.frontmatter.get("excerpt"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let author = page
        .frontmatter
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| site_metadata.author.clone());

    Some(FeedItem {
        title,
        url: full_url,
        date,
        summary,
        author,
    })
}

/// Try to extract and parse a date from frontmatter
pub fn extract_date_from_frontmatter(frontmatter: &serde_yaml::Value) -> Option<DateTime<Utc>> {
    let date_str = frontmatter
        .get("date")
        .or_else(|| frontmatter.get("published"))
        .or_else(|| frontmatter.get("created"))
        .or_else(|| frontmatter.get("pubDate"))
        .and_then(|v| v.as_str())?;

    parse_date_string(date_str)
}

/// Parse a date string in various common formats
fn parse_date_string(s: &str) -> Option<DateTime<Utc>> {
    // ISO 8601 / RFC 3339 (2024-01-15T10:30:00Z)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // YYYY-MM-DD (2024-01-15)
    if let Ok(nd) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let ndt = nd.and_hms_opt(0, 0, 0)?;
        return Some(DateTime::from_naive_utc_and_offset(ndt, Utc));
    }

    // YYYY-MM-DD HH:MM:SS (2024-01-15 10:30:00)
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(ndt, Utc));
    }

    console::warn(format!(
        "couldn't parse date '{}'. Supported: YYYY-MM-DD, YYYY-MM-DDTHH:MM:SSZ, YYYY-MM-DD HH:MM:SS",
        s
    ));
    None
}

/// Generate RSS 2.0 feed XML
pub fn generate_rss(
    items: &[FeedItem],
    feed_config: &FeedConfig,
    site_metadata: &SiteMetadata,
) -> Result<String> {
    let title = feed_config
        .title
        .as_ref()
        .or(site_metadata.title.as_ref())
        .ok_or_else(|| HugsError::FeedMissingTitle {
            feed_name: feed_config.name.clone().into(),
        })?;

    let base_url = site_metadata
        .url
        .as_ref()
        .ok_or_else(|| HugsError::FeedMissingUrl {
            feed_name: feed_config.name.clone().into(),
        })?;

    let description = feed_config
        .description
        .as_ref()
        .or(site_metadata.description.as_ref())
        .cloned()
        .unwrap_or_default();

    let rss_items: Vec<rss::Item> = items
        .iter()
        .map(|item| {
            let mut builder = ItemBuilder::default();
            builder.title(Some(item.title.clone()));
            builder.link(Some(item.url.clone()));
            builder.guid(Some(
                GuidBuilder::default()
                    .value(item.url.clone())
                    .permalink(true)
                    .build(),
            ));

            if let Some(date) = &item.date {
                builder.pub_date(Some(date.to_rfc2822()));
            }

            if let Some(summary) = &item.summary {
                builder.description(Some(summary.clone()));
            }

            if let Some(author) = &item.author {
                builder.author(Some(author.clone()));
            }

            builder.build()
        })
        .collect();

    let channel = ChannelBuilder::default()
        .title(title.clone())
        .link(base_url.clone())
        .description(description)
        .language(Some(site_metadata.language.clone()))
        .generator(Some("Hugs Static Site Generator".to_string()))
        .items(rss_items)
        .build();

    Ok(channel.to_string())
}

/// Generate Atom feed XML
pub fn generate_atom(
    items: &[FeedItem],
    feed_config: &FeedConfig,
    site_metadata: &SiteMetadata,
) -> Result<String> {
    use atom_syndication::{Entry, Feed, Generator, Link, Person, Text};

    let title = feed_config
        .title
        .as_ref()
        .or(site_metadata.title.as_ref())
        .ok_or_else(|| HugsError::FeedMissingTitle {
            feed_name: feed_config.name.clone().into(),
        })?;

    let base_url = site_metadata
        .url
        .as_ref()
        .ok_or_else(|| HugsError::FeedMissingUrl {
            feed_name: feed_config.name.clone().into(),
        })?;

    let entries: Vec<Entry> = items
        .iter()
        .map(|item| {
            let mut entry = Entry::default();
            entry.set_title(Text::plain(&item.title));
            entry.set_id(&item.url);
            entry.set_links(vec![Link {
                href: item.url.clone(),
                rel: "alternate".to_string(),
                ..Default::default()
            }]);

            if let Some(date) = &item.date {
                entry.set_updated(*date);
            } else {
                entry.set_updated(Utc::now());
            }

            if let Some(summary) = &item.summary {
                entry.set_summary(Some(Text::plain(summary)));
            }

            if let Some(author) = &item.author {
                entry.set_authors(vec![Person {
                    name: author.clone(),
                    ..Default::default()
                }]);
            }

            entry
        })
        .collect();

    let mut feed = Feed::default();
    feed.set_title(Text::plain(title));
    feed.set_id(base_url);
    feed.set_links(vec![Link {
        href: base_url.clone(),
        rel: "alternate".to_string(),
        ..Default::default()
    }]);
    feed.set_updated(Utc::now());
    feed.set_generator(Some(Generator {
        value: "Hugs Static Site Generator".to_string(),
        ..Default::default()
    }));
    feed.set_entries(entries);

    Ok(feed.to_string())
}
