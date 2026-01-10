use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod build;
mod config;
mod console;
mod dev;
mod doc;
mod error;
mod feed;
mod highlight;
mod minify;
mod new;
mod run;
mod sitemap;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the development server with live reloading
    Dev {
        /// Path to the site directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Port to run on (if specified, fails when unavailable; otherwise retries)
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Build the static site
    Build {
        /// Path to the site directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output directory for the built site
        #[arg(short, long, default_value = "dist")]
        output: PathBuf,
    },
    /// Create a new Hugs site
    #[command(after_help = "If you don't provide a name, I'll ask you for one!")]
    New {
        /// Name for your new site folder (created in current directory)
        name: Option<PathBuf>,
    },
    /// Open the Hugs documentation in your browser
    Doc {
        /// Port to run the documentation server on
        #[arg(short, long)]
        port: Option<u16>,

        /// Don't automatically open the browser
        #[arg(long)]
        no_open: bool,
    },
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .context_lines(3)
                .rgb_colors(miette::RgbColors::Preferred)
                .color(true)
                .with_syntax_highlighting(miette::highlighters::SyntectHighlighter::default())
                .build(),
        )
    }))
    .expect("Failed to set miette hook");

    let args = Args::parse();

    match args.command {
        Command::Dev { path, port } => {
            let port_explicit = port.is_some();
            let port = port.unwrap_or(8080);
            crate::dev::run_dev_server(path, port, port_explicit).await?;
        }
        Command::Build { path, output } => {
            crate::build::run_build(path, output).await?;
        }
        Command::New { name } => {
            crate::new::create_site(name).await?;
        }
        Command::Doc { port, no_open } => {
            crate::doc::run_doc_server(port, no_open).await?;
        }
    }

    Ok(())
}
