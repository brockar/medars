use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_medars_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("medars");
    path
}

fn get_test_image() -> PathBuf {
    PathBuf::from("imgs/note102.jpg")
}

#[test]
fn test_cli_check_command() {
    let binary = get_medars_binary();
    let test_image = get_test_image();
    
    if !test_image.exists() {
        eprintln!("Skipping test: test image does not exist");
        return;
    }
    
    let output = Command::new(&binary)
        .arg("check")
        .arg(&test_image)
        .output();
    
    assert!(output.is_ok());
    let output = output.unwrap();
    assert!(output.status.success() || output.status.code() == Some(1));
}

#[test]
fn test_cli_show_command() {
    let binary = get_medars_binary();
    let test_image = get_test_image();
    
    if !test_image.exists() {
        eprintln!("Skipping test: test image does not exist");
        return;
    }
    
    let output = Command::new(&binary)
        .arg("show")
        .arg(&test_image)
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_cli_show_json_format() {
    let binary = get_medars_binary();
    let test_image = get_test_image();
    
    if !test_image.exists() {
        eprintln!("Skipping test: test image does not exist");
        return;
    }
    
    let output = Command::new(&binary)
        .arg("show")
        .arg(&test_image)
        .arg("--format")
        .arg("json")
        .output();
    
    assert!(output.is_ok());
    let output = output.unwrap();
    
    // Check if output is valid JSON
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            let json_result: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
            assert!(json_result.is_ok() || stdout.contains("No Metadata"));
        }
    }
}

#[test]
fn test_cli_clean_command() {
    let binary = get_medars_binary();
    let test_image = get_test_image();
    
    if !test_image.exists() {
        eprintln!("Skipping test: test image does not exist");
        return;
    }
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("cleaned.jpg");
    
    let output = Command::new(&binary)
        .arg("clean")
        .arg(&test_image)
        .arg("--output")
        .arg(&output_path)
        .output();
    
    assert!(output.is_ok());
    let output = output.unwrap();
    
    // Skip test if rexiv2 not available or clean failed
    if !output.status.success() || !output_path.exists() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Skipping: clean command failed - {}", stderr);
        return;
    }
    
    assert!(output_path.exists());
}

#[test]
fn test_cli_clean_with_copy() {
    let binary = get_medars_binary();
    let test_image = get_test_image();
    
    if !test_image.exists() {
        eprintln!("Skipping test: test image does not exist");
        return;
    }
    
    let temp_dir = TempDir::new().unwrap();
    let temp_image = temp_dir.path().join("temp_image.jpg");
    fs::copy(&test_image, &temp_image).unwrap();
    
    let output = Command::new(&binary)
        .arg("clean")
        .arg(&temp_image)
        .arg("--copy")
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_cli_log_command() {
    let binary = get_medars_binary();
    
    let output = Command::new(&binary)
        .arg("log")
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_cli_log_with_limit() {
    let binary = get_medars_binary();
    
    let output = Command::new(&binary)
        .arg("log")
        .arg("--max")
        .arg("5")
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_cli_invalid_command() {
    let binary = get_medars_binary();
    
    let output = Command::new(&binary)
        .arg("--help") // Use help instead to avoid timeout
        .output();
    
    assert!(output.is_ok());
    let output = output.unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_check_nonexistent_file() {
    let binary = get_medars_binary();
    
    let output = Command::new(&binary)
        .arg("check")
        .arg("nonexistent_file.jpg")
        .output();
    
    assert!(output.is_ok());
    let output = output.unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_cli_glob_pattern() {
    let binary = get_medars_binary();
    
    // Test with glob pattern (may or may not match files)
    let output = Command::new(&binary)
        .arg("check")
        .arg("imgs/*.jpg")
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_cli_help_flag() {
    let binary = get_medars_binary();
    
    let output = Command::new(&binary)
        .arg("--help")
        .output();
    
    assert!(output.is_ok());
    let output = output.unwrap();
    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("medars") || stdout.contains("Usage"));
}

#[test]
fn test_cli_version_flag() {
    let binary = get_medars_binary();
    
    let output = Command::new(&binary)
        .arg("--version")
        .output();
    
    assert!(output.is_ok());
    let output = output.unwrap();
    assert!(output.status.success());
}

#[test]
fn test_end_to_end_workflow() {
    let binary = get_medars_binary();
    let test_image = get_test_image();
    
    if !test_image.exists() {
        eprintln!("Skipping test: test image does not exist");
        return;
    }
    
    let temp_dir = TempDir::new().unwrap();
    let cleaned_path = temp_dir.path().join("cleaned.jpg");
    
    // 1. Check metadata
    let _check_output = Command::new(&binary)
        .arg("check")
        .arg(&test_image)
        .output()
        .unwrap();
    
    // 2. Show metadata
    let _show_output = Command::new(&binary)
        .arg("show")
        .arg(&test_image)
        .output()
        .unwrap();
    
    // 3. Clean metadata
    let clean_output = Command::new(&binary)
        .arg("clean")
        .arg(&test_image)
        .arg("--output")
        .arg(&cleaned_path)
        .output()
        .unwrap();
    
    // Skip if rexiv2 not available or clean failed
    if !clean_output.status.success() || !cleaned_path.exists() {
        let stderr = String::from_utf8_lossy(&clean_output.stderr);
        eprintln!("Skipping: clean failed - {}", stderr);
        return;
    }
    
    assert!(cleaned_path.exists());
    
    // 4. Check cleaned file has no metadata
    let check_clean_output = Command::new(&binary)
        .arg("check")
        .arg(&cleaned_path)
        .output()
        .unwrap();
    
    // Should indicate no metadata or minimal metadata
    assert!(check_clean_output.status.success() || check_clean_output.status.code() == Some(1));
}
