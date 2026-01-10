use std::borrow::Cow;
use std::path::PathBuf;

use dialoguer::{theme::ColorfulTheme, Input};
use include_dir::{include_dir, Dir};
use owo_colors::OwoColorize;
use tokio::fs;

use crate::console;
use crate::error::{HugsError, Result, StyledPath};

/// The template directory embedded at compile time
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/tutorial-site");

/// Create a new Hugs site at the given path
pub async fn create_site(name: Option<PathBuf>) -> Result<()> {
    let path = match name {
        Some(p) => p,
        None => {
            let name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("What would you like to name your site?")
                .interact_text()
                .map_err(|e| HugsError::InputError { cause: e.to_string() })?;
            PathBuf::from(name)
        }
    };
    // Check if directory exists and is non-empty
    if path.exists() {
        let mut entries = fs::read_dir(&path).await.map_err(|e| HugsError::FileRead {
            path: StyledPath::from(&path),
            cause: e,
        })?;

        if entries
            .next_entry()
            .await
            .map_err(|e| HugsError::FileRead {
                path: StyledPath::from(&path),
                cause: e,
            })?
            .is_some()
        {
            return Err(HugsError::DirNotEmpty {
                path: StyledPath::from(&path),
            });
        }
    }

    console::status("Creating", format!("new site at {}", path.display()));

    extract_dir(&TEMPLATE_DIR, &path).await?;

    // Print success message
    let path_display = path.display().to_string();
    let path_quoted = shell_quote(&path_display);
    println!();
    println!(
        "  {} Created new Hugs site at {}",
        "✓".green().bold(),
        path_display.cyan()
    );
    println!();
    println!("  To get started:");
    println!("    {} {}", "cd".bold(), path_quoted);
    println!("    {}", "hugs dev .".bold());
    println!();

    Ok(())
}

/// Quote a string for shell usage if it contains special characters
fn shell_quote(s: &str) -> Cow<'_, str> {
    let needs_quoting = s.is_empty()
        || s.chars()
            .any(|c| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/'));

    if needs_quoting {
        // Use double quotes, escaping $ ` \ " and !
        let escaped: String = s
            .chars()
            .flat_map(|c| match c {
                '$' | '`' | '\\' | '"' | '!' => vec!['\\', c],
                _ => vec![c],
            })
            .collect();
        Cow::Owned(format!("\"{}\"", escaped))
    } else {
        Cow::Borrowed(s)
    }
}

/// Recursively extract an embedded directory to the filesystem
async fn extract_dir(dir: &Dir<'_>, target: &PathBuf) -> Result<()> {
    // Create the target directory
    fs::create_dir_all(target)
        .await
        .map_err(|e| HugsError::CreateDir {
            path: StyledPath::from(target),
            cause: e,
        })?;

    // Process all entries
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(subdir) => {
                let subdir_path = target.join(subdir.path().file_name().unwrap());
                Box::pin(extract_dir(subdir, &subdir_path)).await?;
            }
            include_dir::DirEntry::File(file) => {
                let file_path = target.join(file.path().file_name().unwrap());
                fs::write(&file_path, file.contents())
                    .await
                    .map_err(|e| HugsError::FileWrite {
                        path: StyledPath::from(&file_path),
                        cause: e,
                    })?;
            }
        }
    }

    Ok(())
}
