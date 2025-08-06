use std::{collections::HashMap, fs::File, io::BufReader, path::Path};
use anyhow::{Context, Result};
use exif;


pub struct MetadataHandler;

impl MetadataHandler {
    pub fn new() -> Self {
        Self
    }

    /// Get a formatted metadata table as a String
    pub fn get_metadata_table(&self, path: &Path) -> Result<String> {
        let metadata = self.extract_metadata(path)?;
        if metadata.is_empty() {
            return Ok("❌ No metadata found".to_string());
        }
        let mut table = String::from("📋 Image Metadata:\n");
        table.push_str(&"─".repeat(40));
        table.push('\n');
        for (key, value) in &metadata {
            table.push_str(&format!("{}: {}\n", key, value));
        }
        table.push_str(&"─".repeat(40));
        table.push('\n');
        table.push_str(&format!("📊 Total metadata fields: {}", metadata.len()));
        Ok(table)
    }

    /// Check if an image has any metadata
    pub fn has_metadata(&self, path: &Path) -> Result<bool> {
        if !path.exists() {
            anyhow::bail!("File does not exist: {}", path.display());
        }
        let file = File::open(path)?;
        let mut bufreader = BufReader::new(&file);
        match exif::Reader::new().read_from_container(&mut bufreader) {
            Ok(exif_data) => Ok(exif_data.fields().count() > 0),
            Err(_) => self.check_other_metadata(path),
        }
    }

    /// Display metadata in the specified format ("json" or "table")
    pub fn display_metadata(&self, path: &Path, format: &str, quiet: bool) -> Result<()> {
        if !path.exists() {
            anyhow::bail!("File does not exist: {}", path.display());
        }
        let metadata = self.extract_metadata(path)?;
        match format.to_lowercase().as_str() {
            "json" => self.display_json(&metadata, quiet)?,
            _ => self.display_table(&metadata, quiet)?,
        }
        Ok(())
    }

    /// Remove all metadata from an image and save to output_path
    pub fn remove_metadata(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        if !input_path.exists() {
            anyhow::bail!("Input file does not exist: {}", input_path.display());
        }
        let image = rexiv2::Metadata::new_from_path(input_path)
            .context("Failed to open image with rexiv2")?;
        image.clear();
        image.save_to_file(output_path)
            .context("Failed to save image without metadata using rexiv2")?;
        Ok(())
    }

    /// Extract all available metadata from an image
    fn extract_metadata(&self, path: &Path) -> Result<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        // EXIF
        if let Ok(exif_data) = self.extract_exif_metadata(path) {
            metadata.extend(exif_data);
        }
        // File info
        if let Ok(file_metadata) = std::fs::metadata(path) {
            metadata.insert("File Size".to_string(), format!("{} bytes", file_metadata.len()));
            if let Ok(modified) = file_metadata.modified() {
                metadata.insert("Modified".to_string(), format!("{:?}", modified));
            }
        }
        // Dimensions
        if let Ok(meta) = rexiv2::Metadata::new_from_path(path) {
            let width = meta.get_pixel_width();
            let height = meta.get_pixel_height();
            if width > 0 && height > 0 {
                metadata.insert("Dimensions".to_string(), format!("{}x{}", width, height));
            }
        }
        Ok(metadata)
    }

    /// Extract EXIF metadata only
    fn extract_exif_metadata(&self, path: &Path) -> Result<HashMap<String, String>> {
        let file = File::open(path)?;
        let mut bufreader = BufReader::new(&file);
        let mut metadata = HashMap::new();
        if let Ok(exif_data) = exif::Reader::new().read_from_container(&mut bufreader) {
            for f in exif_data.fields() {
                let tag_name = format!("{}", f.tag);
                let value = f.display_value().with_unit(&exif_data).to_string();
                metadata.insert(tag_name, value);
            }
        }
        Ok(metadata)
    }

    /// Check for other metadata using rexiv2 (returns false if no EXIF)
    fn check_other_metadata(&self, path: &Path) -> Result<bool> {
        match rexiv2::Metadata::new_from_path(path) {
            Ok(_) => Ok(false),
            Err(_) => Ok(false),
        }
    }

    /// Display metadata as a table in stdout
    fn display_table(&self, metadata: &HashMap<String, String>, quiet: bool) -> Result<()> {
        let has_exif = metadata.keys().any(|k| k != "File Size" && k != "Modified" && k != "Dimensions");
        if !has_exif {
            if !quiet {
                eprintln!("No metadata in this image.");
                if let Some(size) = metadata.get("File Size") {
                    println!("File Size: {}", size);
                }
                if let Some(modified) = metadata.get("Modified") {
                    println!("Modified: {}", modified);
                }
                if let Some(dim) = metadata.get("Dimensions") {
                    println!("Dimensions: {}", dim);
                }
            }
            return Ok(());
        }
        if !quiet {
            println!("📋 Image Metadata:");
            println!("{}", "─".repeat(60));
            for (key, value) in metadata {
                println!("{}: {}", key, value);
            }
            println!("{}", "─".repeat(60));
            println!("📊 Total metadata fields: {}", metadata.len());
        }
        Ok(())
    }

    /// Display metadata as pretty JSON in stdout
    fn display_json(&self, metadata: &HashMap<String, String>, quiet: bool) -> Result<()> {
        if !quiet {
            if metadata.is_empty() {
                eprintln!("⚠️  No Metadata found in image");
            } else {
                println!("{}", serde_json::to_string_pretty(metadata)?);
            }
        }
        Ok(())
    }
}