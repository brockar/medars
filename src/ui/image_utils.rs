use std::collections::HashMap;
use crate::metadata::MetadataHandler;

/// Utility struct for image-related (non-TUI) logic
pub struct ImageUtils {
    pub metadata_handler: MetadataHandler,
    pub cached_metadata: Option<(String, String)>, // (filename, formatted_metadata)
}

// Sensitivity classification 
pub const RED_KEYS: [&str; 27] = [
    "GPSLatitude", "GPSLongitude", "GPSAltitude", "GPSLatitudeRef", "GPSLongitudeRef", "GPSAltitudeRef",
    "DateTimeOriginal", "DateTimeDigitized", "DateTime", "OffsetTime", "OffsetTimeOriginal", "OffsetTimeDigitized", 
    "Modified", "GPSTimeStamp", "GPSSpeedRef","GPSDateStamp", "GPSProcessingMethod", "GPSSpeed", "GPSTrack", "GPSImgDirection", 
    "ImageUniqueID", "SubSecTime", "SubSecTimeDigitized", "SubSecTimeOriginal", "ExposureIndex", "LensModel", "MakerNote"
];

pub const YELLOW_KEYS: [&str; 65] = [
    "Make", "Model", "Software", "SceneCaptureType", "DigitalZoomRatio", "FNumber", "ExposureBiasValue",
    "ExposureMode", "MeteringMode", "ShutterSpeedValue", "ExposureTime", "WhiteBalance", "ApertureValue",
    "FocalLength", "FocalLengthIn35mmFilm", "PhotographicSensitivity", "Flash", "ExposureProgram", "ExifVersion",
    "MaxApertureValue", "SceneType", "BrightnessValue", "SensingMethod", "ComponentsConfiguration", 
    "LightSource", "FlashpixVersion", "InteroperabilityIndex", "InteroperabilityVersion", "HostComputer",
    "Tag(Exif, 34953)", "Tag(Exif, 42593)", "Tag(Exif, 34965)", "Tag(Tiff, 39424)", "Tag(Exif, 39321)", 
    "Tag(Tiff, 34970)", "Tag(Tiff, 34979)", "Tag(Exif, 34974)", "Tag(Exif, 39424)", "Tag(Tiff, 39321)",
    "Artist", "Copyright", "ImageDescription", "UserComment", "DocumentName", "PageName",
    "LensMake", "LensSerialNumber", "LensSpecification",
    "SubjectDistance", "SubjectDistanceRange", "Contrast", "Saturation", "Sharpness",
    "GainControl", "CustomRendered", "CompositeImage", "RelatedSoundFile",
    "WaterDepth", "Acceleration", "CameraElevationAngle", 
    "Keywords", "Caption", "Credit", "Byline", "LocationCreated"
];

pub const GREEN_KEYS: [&str; 22] = [
    "PixelXDimension", "PixelYDimension", "ImageWidth", "ImageLength", "Dimensions", "Compression", "ColorSpace",
    "XResolution", "YResolution", "ResolutionUnit", "YCbCrPositioning", "JPEGInterchangeFormat", 
    "JPEGInterchangeFormatLength", "File Size", "Orientation",
    "BitsPerSample", "PhotometricInterpretation", "PlanarConfiguration", "TransferFunction",
    "WhitePoint", "PrimaryChromaticities", "ColorMap"
];

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
        let mut count_red = 0;
        let mut count_yellow = 0;
        let mut count_green = 0;
        let mut count_unrec = 0;
        for key in metadata.keys() {
            if RED_KEYS.contains(&key.as_str()) {
                count_red += 1;
            } else if YELLOW_KEYS.contains(&key.as_str()) {
                count_yellow += 1;
            } else if GREEN_KEYS.contains(&key.as_str()) {
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
            let category = if RED_KEYS.contains(&key.as_str()) {
                "🔴"
            } else if YELLOW_KEYS.contains(&key.as_str()) {
                "🟡"
            } else if GREEN_KEYS.contains(&key.as_str()) {
                "🟢"
            } else {
                "⚪"
            };

            // Try to pretty-print JSON values, including double-quoted/escaped JSON strings
            let trimmed = value.trim();
            let try_json = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() > 2 {
                let unquoted = &trimmed[1..trimmed.len()-1];
                let unescaped = unquoted.replace("\\\"", "\"");
                serde_json::from_str::<serde_json::Value>(&unescaped).ok()
            } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
                serde_json::from_str::<serde_json::Value>(trimmed).ok()
            } else {
                None
            };

            let pretty_value = if let Some(json) = try_json {
                // Indent all lines by two spaces for top-level JSON object
                let pretty = Self::pretty_json_value(&json, 0);
                if json.is_object() {
                    pretty
                        .lines()
                        .map(|line| format!("---   {}", line))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    pretty
                }
            } else if value.len() > 120 {
                format!("{}...", &value[..120])
            } else {
                value.clone()
            };

            if pretty_value.contains('\n') {
                result.push_str(&format!("{} {}:\n{}\n", category, key, pretty_value));
            } else {
                result.push_str(&format!("{} {}: {}\n", category, key, pretty_value));
            }
        }
        result.push_str(&"─".repeat(40));
        result
    }

    /// Recursively pretty-print JSON values for TUI
    fn pretty_json_value(value: &serde_json::Value, indent: usize) -> String {
    let pad = " ".repeat(indent);
        match value {
            serde_json::Value::Object(map) => {
                let mut s = String::new();
                for (k, v) in map {
                    // Indent the sub-key itself
                    s.push_str(&format!("{}{}: ", pad, k));
                    let val_str = Self::pretty_json_value(v, indent);
                    if v.is_object() || v.is_array() {
                        s.push('\n');
                        s.push_str(&val_str);
                        s.push('\n');
                    } else {
                        s.push_str(&format!("{}\n", val_str.trim_end()));
                    }
                }
                s.trim_end_matches('\n').to_string()
            }
            serde_json::Value::Array(arr) => {
                let mut s = String::new();
                for v in arr {
                    s.push_str(&format!("{}- ", pad));
                    let val_str = Self::pretty_json_value(v, indent);
                    if v.is_object() || v.is_array() {
                        s.push('\n');
                        s.push_str(&val_str);
                        s.push('\n');
                    } else {
                        s.push_str(&format!("{}\n", val_str.trim_end()));
                    }
                }
                s.trim_end_matches('\n').to_string()
            }
            _ => format!("{}{}", pad, value),
        }
    }
}
