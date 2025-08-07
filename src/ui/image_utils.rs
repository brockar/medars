use std::collections::HashMap;
use crate::metadata::MetadataHandler;

/// Utility struct for image-related (non-TUI) logic
pub struct ImageUtils {
    pub metadata_handler: MetadataHandler,
    pub cached_metadata: Option<(String, String)>, // (filename, formatted_metadata)
}

impl ImageUtils {
    pub fn new() -> Self {
        ImageUtils {
            metadata_handler: MetadataHandler::new(),
            cached_metadata: None,
        }
    }

    /// Get metadata for display, using cache to avoid re-reading on every frame
    pub fn get_metadata_for_display(&mut self, selected_file: &str, file_path: &std::path::Path) -> String {
        if let Some((cached_filename, cached_text)) = &self.cached_metadata {
            if cached_filename == selected_file {
                return cached_text.clone();
            }
        }
        let metadata_text = match self.metadata_handler.get_metadata_map(file_path) {
            Ok(metadata) => Self::format_metadata_for_tui(&metadata),
            Err(_) => format!("Error reading metadata for: {}", selected_file),
        };
        self.cached_metadata = Some((selected_file.to_string(), metadata_text.clone()));
        metadata_text
    }

    /// Format metadata for TUI display similar to CLI table format
    pub fn format_metadata_for_tui(metadata: &HashMap<String, String>) -> String {
        let has_exif = metadata.keys().any(|k| k != "File Size" && k != "Modified" && k != "Dimensions");
        if !has_exif {
            let mut result = String::from("No metadata in this image.\n");
            if let Some(size) = metadata.get("File Size") {
                result.push_str(&format!("File Size: {}\n", size));
            }
            if let Some(modified) = metadata.get("Modified") {
                result.push_str(&format!("Modified: {}\n", modified));
            }
            if let Some(dim) = metadata.get("Dimensions") {
                result.push_str(&format!("Dimensions: {}\n", dim));
            }
            return result;
        }
        // Sensitivity classification 
        let red_keys = [
            "GPSLatitude", "GPSLongitude", "GPSAltitude", "GPSLatitudeRef", "GPSLongitudeRef", "GPSAltitudeRef",
            "DateTimeOriginal", "DateTimeDigitized", "DateTime", "OffsetTime", "OffsetTimeOriginal", "OffsetTimeDigitized", 
            "Modified", "GPSTimeStamp", "GPSSpeedRef","GPSDateStamp", "GPSProcessingMethod", "GPSSpeed", "GPSTrack", "GPSImgDirection", 
            "ImageUniqueID", "SubSecTime", "SubSecTimeDigitized", "SubSecTimeOriginal", "ExposureIndex", "LensModel",
        ];
        let yellow_keys = [
            "Make", "Model", "Software", "SceneCaptureType", "DigitalZoomRatio", "FNumber", "ExposureBiasValue",
            "ExposureMode", "MeteringMode", "ShutterSpeedValue", "ExposureTime", "WhiteBalance", "ApertureValue",
            "FocalLength", "FocalLengthIn35mmFilm", "PhotographicSensitivity", "Flash", "ExposureProgram", "ExifVersion",
            "MaxApertureValue", "SceneType", "BrightnessValue", "SensingMethod", "ComponentsConfiguration", 
            "LightSource", "FlashpixVersion", "InteroperabilityIndex", "InteroperabilityVersion", 
            "Tag(Exif, 34953)", "Tag(Exif, 42593)", "Tag(Exif, 34965)", "Tag(Tiff, 39424)", "Tag(Exif, 39321)", 
            "Tag(Tiff, 34970)", "Tag(Tiff, 34979)", "Tag(Exif, 34974)", "Tag(Exif, 39424)", "Tag(Tiff, 39321)"
        ];
        let green_keys = [
            "PixelXDimension", "PixelYDimension", "ImageWidth", "ImageLength", "Dimensions", "Compression", "ColorSpace",
            "XResolution", "YResolution", "ResolutionUnit", "YCbCrPositioning", "JPEGInterchangeFormat", 
            "JPEGInterchangeFormatLength", "File Size", "Orientation"
        ];
        let mut count_red = 0;
        let mut count_yellow = 0;
        let mut count_green = 0;
        let mut count_unrec = 0;
        for key in metadata.keys() {
            if red_keys.contains(&key.as_str()) {
                count_red += 1;
            } else if yellow_keys.contains(&key.as_str()) {
                count_yellow += 1;
            } else if green_keys.contains(&key.as_str()) {
                count_green += 1;
            } else {
                count_unrec += 1;
            }
        }
        let total = count_red + count_yellow + count_green + count_unrec;
        let mut result = String::new();
        result.push_str(&"─".repeat(40));
        result.push('\n');
        result.push_str(&format!("🔴 Insecure: {}\n", count_red));
        result.push_str(&format!("🟡 Better to remove: {}\n", count_yellow));
        result.push_str(&format!("🟢 Safe to share: {}\n", count_green));
        if count_unrec > 0 {
            result.push_str(&format!("⚪ Unrecognized: {}\n", count_unrec));
        }
        result.push_str(&format!("📊 Total metadata fields: {}\n", total));
        result.push_str(&"─".repeat(40));
        result.push('\n');
        result.push_str("📋 Image Metadata:\n");
        let mut sorted_entries: Vec<_> = metadata.iter().collect();
        sorted_entries.sort_by_key(|(key, _)| key.as_str());
        for (key, value) in sorted_entries {
            let category = if red_keys.contains(&key.as_str()) {
                "🔴"
            } else if yellow_keys.contains(&key.as_str()) {
                "🟡"
            } else if green_keys.contains(&key.as_str()) {
                "🟢"
            } else {
                "⚪"
            };
            // Try to pretty-print JSON values, else truncate long lines
            let pretty_value = if value.trim_start().starts_with('{') || value.trim_start().starts_with('[') {
                match serde_json::from_str::<serde_json::Value>(value) {
                    Ok(json) => {
                        let pretty = serde_json::to_string_pretty(&json).unwrap_or_else(|_| value.clone());
                        // Indent each line for TUI
                        pretty.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
                    },
                    Err(_) => {
                        // Not valid JSON, fallback to truncation
                        if value.len() > 120 {
                            format!("{}...", &value[..120])
                        } else {
                            value.clone()
                        }
                    }
                }
            } else if value.len() > 120 {
                format!("{}...", &value[..120])
            } else {
                value.clone()
            };
            // If pretty_value contains newlines, print key on first line, value on next lines
            if pretty_value.contains('\n') {
                result.push_str(&format!("{} {}:\n{}\n", category, key, pretty_value));
            } else {
                result.push_str(&format!("{} {}: {}\n", category, key, pretty_value));
            }
        }
        result.push_str(&"─".repeat(40));
        result
    }
}
