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
        /// Copy to new file (optional path, or auto-name if not provided)
        #[arg(long, value_name = "COPY_PATH")]
        copy: Option<Option<PathBuf>>,
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
            Commands::Remove { file, output, copy } => {
                let handler = MetadataHandler::new();
                let output_path = if let Some(copy_flag) = copy {
                    match copy_flag {
                        Some(path) => path.clone(),
                        None => {
                            // Auto-generate output path: original stem + _medars + ext
                            let orig = file;
                            let parent = orig.parent();
                            let stem = orig.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                            let ext = orig.extension().and_then(|e| e.to_str()).unwrap_or("");
                            let mut new_name = format!("{}_medars", stem);
                            if !ext.is_empty() {
                                new_name.push('.');
                                new_name.push_str(ext);
                            }
                            if let Some(parent) = parent {
                                parent.join(new_name)
                            } else {
                                PathBuf::from(new_name)
                            }
                        }
                    }
                } else {
                    output.as_ref().cloned().unwrap_or_else(|| file.clone())
                };
                if let Some(parent) = output_path.parent() {
                    // Only try to create if parent is not empty and not "." (current dir)
                    if parent != std::path::Path::new("") && parent != std::path::Path::new(".") && !parent.exists() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            log::error!("Failed to create output directory {}: {}", parent.display(), e);
                            eprintln!("Failed to create output directory {}: {}", parent.display(), e);
                            return Err(e.into());
                        }
                    }
                }
                // If --copy is used, copy the input file to the output path first
                if copy.is_some() {
                    // Only copy if output_path != file
                    if output_path != *file {
                        std::fs::copy(&file, &output_path)?;
                    }
                }
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
