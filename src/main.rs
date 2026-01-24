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
#[command(
    about = "A cozy static site generator (っ◕‿◕)っ",
    after_help = "I'm here to help you build beautiful static sites! Run `hugs <command> --help` for more info on a specific command.",
    subcommand_help_heading = "What can I do for you",
    disable_help_subcommand = true,
    disable_help_flag = true,
)]
struct Args {
    /// Show this help message
    #[arg(short, long, action = clap::ArgAction::Help, global = true)]
    help: (),

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// I'll run a development server with live reloading
    Dev {
        /// Path to the site directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Port to run on (if specified, I'll fail when unavailable; otherwise I'll retry)
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// I'll build your static site
    Build {
        /// Path to the site directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output directory for the built site
        #[arg(short, long, default_value = "dist")]
        output: PathBuf,
    },
    /// I'll create a new Hugs site for you
    #[command(after_help = "If you don't provide a name, I'll ask you for one!")]
    New {
        /// Name for your new site folder (I'll create it in the current directory)
        name: Option<PathBuf>,
    },
    /// I'll open the Hugs documentation in your browser
    Doc {
        /// Port to run the documentation server on
        #[arg(short, long)]
        port: Option<u16>,

        /// Don't automatically open the browser
        #[arg(long)]
        no_open: bool,

        /// I'll extract docs to a folder and print the path (useful for giving LLMs context)
        #[arg(long, num_args = 0..=1)]
        dump: Option<Option<PathBuf>>,
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
            crate::dev::run_dev_server(path, port).await?;
        }
        Command::Build { path, output } => {
            crate::build::run_build(path, output).await?;
        }
        Command::New { name } => {
            crate::new::create_site(name).await?;
        }
        Command::Doc { port, no_open, dump } => {
            if let Some(maybe_path) = dump {
                crate::doc::dump_docs(maybe_path).await?;
            } else {
                crate::doc::run_doc_server(port, no_open).await?;
            }
        }
    }

    Ok(())
}
