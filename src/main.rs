use clap::{Parser, Subcommand};
use std::path::PathBuf;
mod metadata;
use metadata::MetadataHandler;
mod ui;
use ui::RatatuiUI;

#[derive(Parser)]
#[command(name = "medars")]
#[command(about = "Inspect, view, or strip metadata from images — fast and easy. (Also works in TUI!)")]
#[command(version = "0.1.0")]

struct Cli {
    /// Suppress output
    #[arg(short, long, global = true)]
    quiet: bool,
    /// Image file to inspect
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check if an image contains metadata
    Check {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// View metadata in a readable format
    View {
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Output format (json, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Remove metadata from an image
    Remove {
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Output file path (if not specified, overwrites original)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Launch interactive mode (TUI)
    Interactive {
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // If a subcommand is provided, handle as usual
    if let Some(command) = &cli.command {
        if let Commands::Interactive { file } = command {
            let mut ui = RatatuiUI::new();
            if !cli.quiet {
                ui.run(file.clone()).await?;
            }
            return Ok(());
        }

        match command {
            Commands::Check { file } => {
                let handler = MetadataHandler::new();
                let has_metadata = handler.has_metadata(&file)?;
                if !cli.quiet {
                    if has_metadata {
                        log::info!("❌ Image contains metadata");
                        println!("❌ Image contains metadata");
                    } else {
                        log::warn!("✅ No metadata found in image");
                        eprintln!("✅ No metadata found in image");
                    }
                }
            }
            Commands::View { file, format } => {
                let handler = MetadataHandler::new();
                if let Err(e) = handler.display_metadata(&file, &format, cli.quiet) {
                    log::error!("Error: {}", e);
                    eprintln!("Error: {}", e);
                }
            }
            Commands::Remove { file, output } => {
                let handler = MetadataHandler::new();
                let output_path = output.as_ref().cloned().unwrap_or_else(|| file.clone());
                handler.remove_metadata(&file, &output_path)?;
                if !cli.quiet {
                    log::info!("✅ Metadata removed successfully, saved on: {}", output_path.display());
                    println!("✅ Metadata removed successfully, saved on: {}", output_path.display());
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // If no subcommand but a file is provided, run interactive mode
    if cli.file.is_some() {
        let mut ui = RatatuiUI::new();
        if !cli.quiet {
            ui.run(cli.file.clone()).await?;
        }
        return Ok(());
    }
    Ok(())
}
