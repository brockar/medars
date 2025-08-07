use clap::{Parser, Subcommand};
use std::path::PathBuf;
mod metadata;
use metadata::MetadataHandler;
mod ui;
use ui::RatatuiUI;
mod logger;
use logger::{Logger, LogEntry};

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
    Show {
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Output format (json, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Clean metadata from an image
    Clean {
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Output file path (if not specified, overwrites original)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Copy to new file (optional path, or auto-name if not provided)
        #[arg(long, value_name = "COPY_PATH")]
        copy: Option<Option<PathBuf>>,
        /// Show what would be removed, but do not modify the file
        #[arg(long)]
        dry_run: bool,
    },

    /// Show recent log entries
    Log {
        /// Maximum number of entries to show
        #[arg(short, long)]
        max: Option<usize>,
    },

    /// Launch interactive mode (TUI)
    Tui {
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let logger = Logger::new();

    // If a subcommand is provided, handle as usual
    if let Some(command) = &cli.command {
        if let Commands::Tui { file } = command {
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
            Commands::Show { file, format } => {
                let handler = MetadataHandler::new();
                if let Err(e) = handler.display_metadata(&file, &format, cli.quiet) {
                    log::error!("Error: {}", e);
                    eprintln!("Error: {}", e);
                }
            }
            Commands::Clean { file, output, copy, dry_run } => {
                let handler = MetadataHandler::new();
                if *dry_run {
                    // Show what would be removed (list all metadata keys)
                    let meta = handler.get_metadata_map(&file)?;
                    if meta.is_empty() {
                        if !cli.quiet {
                            println!("✅ No metadata found in image (nothing to remove)");
                        }
                    } else {
                        if !cli.quiet {
                            println!("The following metadata would be removed from {}:", file.display());
                            for (k, v) in meta.iter() {
                                println!("- {}: {}", k, v);
                            }
                        }
                    }
                    return Ok(());
                }
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
                            // Log failure
                            logger.log(&LogEntry {
                                timestamp: chrono::Utc::now(),
                                action: "remove".to_string(),
                                file: file.display().to_string(),
                                result: "failure".to_string(),
                                details: Some(format!("Failed to create output directory: {}", e)),
                            });
                            return Err(e.into());
                        }
                    }
                }
                if copy.is_some() {
                    // Only copy if output_path != file
                    if output_path != *file {
                        std::fs::copy(&file, &output_path)?;
                    }
                }
                match handler.remove_metadata(&file, &output_path) {
                    Ok(_) => {
                        if !cli.quiet {
                            log::info!("✅ Metadata removed successfully, saved on: {}", output_path.display());
                            println!("✅ Metadata removed successfully, saved on: {}", output_path.display());
                        }
                        logger.log(&LogEntry {
                            timestamp: chrono::Utc::now(),
                            action: "clean".to_string(),
                            file: file.display().to_string(),
                            result: "success".to_string(),
                            details: Some(format!("Saved on: {}", output_path.display())),
                        });
                    }
                    Err(e) => {
                        if !cli.quiet {
                            log::error!("Failed to remove metadata: {}", e);
                            eprintln!("Failed to remove metadata: {}", e);
                        }
                        logger.log(&LogEntry {
                            timestamp: chrono::Utc::now(),
                            action: "clean".to_string(),
                            file: file.display().to_string(),
                            result: "failure".to_string(),
                            details: Some(format!("Error: {}", e)),
                        });
                        return Err(e);
                    }
                }
            }
            Commands::Log { max } => {
                let entries = logger.read_logs(*max);
                if entries.is_empty() {
                    println!("No log entries found.");
                } else {
                    for entry in entries {
                        println!("[{}] {} {} {} {}", entry.timestamp, entry.action, entry.file, entry.result, entry.details.unwrap_or_default());
                    }
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
