use std::path::PathBuf;

use include_dir::{include_dir, Dir};
use owo_colors::OwoColorize;
use tokio::fs;
use tracing::info;

use crate::error::{HugsError, Result, StyledPath};

/// The template directory embedded at compile time
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/tutorial-site");

/// Create a new Hugs site at the given path
pub async fn create_site(path: PathBuf) -> Result<()> {
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

    info!("Creating new site at {}", path.display());

    // Extract embedded template directory
    extract_dir(&TEMPLATE_DIR, &path).await?;

    // Print success message
    let path_display = path.display();
    println!();
    println!(
        "  {} Created new Hugs site at {}",
        "✓".green().bold(),
        path_display.to_string().cyan()
    );
    println!();
    println!("  To get started:");
    println!("    {} {}", "cd".bold(), path_display);
    println!("    {}", "hugs dev .".bold());
    println!();

    Ok(())
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
