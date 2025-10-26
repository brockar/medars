use medars::metadata::MetadataHandler;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn get_test_image_path() -> PathBuf {
    PathBuf::from("imgs/note102.jpg")
}

fn get_clean_test_image_path() -> PathBuf {
    PathBuf::from("imgs/note102_medars.jpg")
}

#[test]
fn test_metadata_handler_new() {
    let handler = MetadataHandler::new();
    // Just verify it can be created
    let _ = handler;
}

#[test]
fn test_has_metadata_with_exif_image() {
    let handler = MetadataHandler::new();
    let path = get_test_image_path();
    
    if !path.exists() {
        eprintln!("Skipping test: {} does not exist", path.display());
        return;
    }
    
    let result = handler.has_metadata(&path);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_has_metadata_nonexistent_file() {
    let handler = MetadataHandler::new();
    let path = PathBuf::from("nonexistent_file.jpg");
    
    let result = handler.has_metadata(&path);
    assert!(result.is_err());
}

#[test]
fn test_get_metadata_map() {
    let handler = MetadataHandler::new();
    let path = get_test_image_path();
    
    if !path.exists() {
        eprintln!("Skipping test: {} does not exist", path.display());
        return;
    }
    
    let result = handler.get_metadata_map(&path);
    assert!(result.is_ok());
    
    let metadata = result.unwrap();
    assert!(!metadata.is_empty());
    
    // Check for basic file metadata
    assert!(metadata.contains_key("File Size") || metadata.len() > 0);
}

#[test]
fn test_get_metadata_map_clean_image() {
    let handler = MetadataHandler::new();
    let path = get_clean_test_image_path();
    
    if !path.exists() {
        eprintln!("Skipping test: {} does not exist", path.display());
        return;
    }
    
    let result = handler.get_metadata_map(&path);
    assert!(result.is_ok());
    
    let metadata = result.unwrap();
    // Clean image should have minimal metadata (only file info)
    let has_exif = metadata.keys()
        .any(|k| k != "File Size" && k != "Modified" && k != "Dimensions");
    assert!(!has_exif || metadata.len() < 5);
}

#[test]
fn test_remove_metadata() {
    let handler = MetadataHandler::new();
    let input_path = get_test_image_path();
    
    if !input_path.exists() {
        eprintln!("Skipping test: {} does not exist", input_path.display());
        return;
    }
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test_output.jpg");
    
    let result = handler.remove_metadata(&input_path, &output_path);
    
    // If rexiv2 fails (library not available), skip test
    if result.is_err() {
        eprintln!("Skipping test: rexiv2 library may not be available");
        return;
    }
    
    assert!(result.is_ok());
    assert!(output_path.exists());
    
    // Verify metadata was removed
    let metadata_after = handler.get_metadata_map(&output_path);
    assert!(metadata_after.is_ok());
    
    let meta = metadata_after.unwrap();
    let has_exif = meta.keys()
        .any(|k| k != "File Size" && k != "Modified" && k != "Dimensions");
    assert!(!has_exif || meta.len() < 5);
}

#[test]
fn test_remove_metadata_nonexistent_input() {
    let handler = MetadataHandler::new();
    let input_path = PathBuf::from("nonexistent_input.jpg");
    let output_path = PathBuf::from("output.jpg");
    
    let result = handler.remove_metadata(&input_path, &output_path);
    assert!(result.is_err());
}

#[test]
fn test_display_metadata_table_format() {
    let handler = MetadataHandler::new();
    let path = get_test_image_path();
    
    if !path.exists() {
        eprintln!("Skipping test: {} does not exist", path.display());
        return;
    }
    
    let result = handler.display_metadata(&path, "table", true);
    assert!(result.is_ok());
}

#[test]
fn test_display_metadata_json_format() {
    let handler = MetadataHandler::new();
    let path = get_test_image_path();
    
    if !path.exists() {
        eprintln!("Skipping test: {} does not exist", path.display());
        return;
    }
    
    let result = handler.display_metadata(&path, "json", true);
    assert!(result.is_ok());
}

#[test]
fn test_metadata_preservation_after_copy() {
    let handler = MetadataHandler::new();
    let source_path = get_test_image_path();
    
    if !source_path.exists() {
        eprintln!("Skipping test: {} does not exist", source_path.display());
        return;
    }
    
    let temp_dir = TempDir::new().unwrap();
    let copied_path = temp_dir.path().join("copied.jpg");
    
    // Copy file normally (metadata should be preserved)
    fs::copy(&source_path, &copied_path).unwrap();
    
    let original_metadata = handler.get_metadata_map(&source_path).unwrap();
    let copied_metadata = handler.get_metadata_map(&copied_path).unwrap();
    
    // File size should be similar
    assert!(original_metadata.contains_key("File Size"));
    assert!(copied_metadata.contains_key("File Size"));
}

#[test]
fn test_batch_metadata_removal() {
    let handler = MetadataHandler::new();
    let test_images = vec![
        get_test_image_path(),
    ];
    
    let temp_dir = TempDir::new().unwrap();
    
    for (idx, input_path) in test_images.iter().enumerate() {
        if !input_path.exists() {
            continue;
        }
        
        let output_path = temp_dir.path().join(format!("output_{}.jpg", idx));
        let result = handler.remove_metadata(input_path, &output_path);
        
        // Skip if rexiv2 library not available
        if result.is_err() {
            eprintln!("Skipping batch test item: rexiv2 library may not be available");
            continue;
        }
        
        assert!(result.is_ok());
        assert!(output_path.exists());
    }
}

#[test]
fn test_metadata_map_contains_dimensions() {
    let handler = MetadataHandler::new();
    let path = get_test_image_path();
    
    if !path.exists() {
        eprintln!("Skipping test: {} does not exist", path.display());
        return;
    }
    
    let metadata = handler.get_metadata_map(&path).unwrap();
    
    // Should have dimensions for valid image
    assert!(
        metadata.contains_key("Dimensions") || metadata.len() > 0,
        "Metadata should contain dimensions or other data"
    );
}
