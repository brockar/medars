# MEDARS Test Suite

## Overview

Comprehensive test suite for the MEDARS metadata inspection and removal tool.

## Test Statistics

- **Total Tests**: 49 tests
- **Unit Tests**: 16 tests (8 in lib.rs, 8 in main.rs)
- **Integration Tests**: 13 tests
- **Logger Tests**: 11 tests
- **Metadata Tests**: 12 tests

## Test Categories

### 1. Unit Tests (`src/`)

Located inline in source files:

#### Logger Module (`src/logger.rs`)

- `test_log_entry_creation_with_all_fields` - Validates LogEntry struct creation
- `test_log_entry_serialization_deserialization` - Tests JSON serialization
- `test_logger_path_creation` - Verifies logger path initialization

#### Metadata Module (`src/metadata.rs`)

- `test_metadata_handler_creation` - Tests MetadataHandler instantiation
- `test_extract_metadata_nonexistent_file` - Handles missing files gracefully
- `test_has_metadata_with_invalid_path` - Error handling for invalid paths
- `test_display_metadata_formats` - Validates format string handling
- `test_remove_metadata_same_input_output` - Prevents overwriting source files

### 2. Integration Tests (`tests/integration_tests.rs`)

End-to-end CLI testing:

#### Command Tests

- `test_cli_check_command` - Tests metadata checking
- `test_cli_show_command` - Validates metadata display
- `test_cli_show_json_format` - JSON output format
- `test_cli_clean_command` - Metadata removal (with rexiv2 fallback)
- `test_cli_clean_with_copy` - Copy mode testing
- `test_cli_log_command` - Log viewing
- `test_cli_log_with_limit` - Log pagination

#### Edge Cases

- `test_cli_check_nonexistent_file` - Error handling for missing files
- `test_cli_glob_pattern` - Wildcard pattern support
- `test_cli_help_flag` - Help documentation
- `test_cli_version_flag` - Version display
- `test_cli_invalid_command` - Invalid command handling

#### Workflows

- `test_end_to_end_workflow` - Complete check → show → clean → verify workflow

### 3. Logger Tests (`tests/logger_tests.rs`)

Logging functionality:

- `test_logger_new` - Logger initialization
- `test_log_entry_creation` - Log entry struct creation
- `test_log_entry_serialization` - JSON serialization
- `test_log_entry_deserialization` - JSON deserialization
- `test_logger_log_write` - Writing log entries
- `test_logger_read_logs_empty` - Reading from empty log
- `test_logger_read_logs_with_limit` - Log pagination
- `test_logger_read_logs_no_limit` - Full log reading
- `test_log_entry_with_details` - Optional details field
- `test_log_entry_without_details` - Null details field
- `test_logger_multiple_actions` - Multiple log operations

### 4. Metadata Tests (`tests/metadata_tests.rs`)

Core metadata operations:

#### Basic Operations

- `test_metadata_handler_new` - Handler creation
- `test_has_metadata_with_exif_image` - EXIF detection
- `test_has_metadata_nonexistent_file` - Error handling
- `test_get_metadata_map` - Metadata extraction
- `test_get_metadata_map_clean_image` - Clean image handling

#### Display & Format

- `test_display_metadata_table_format` - Table output
- `test_display_metadata_json_format` - JSON output
- `test_metadata_map_contains_dimensions` - Dimension extraction

#### Removal Operations

- `test_remove_metadata` - Metadata removal (with rexiv2 fallback)
- `test_remove_metadata_nonexistent_input` - Error handling
- `test_batch_metadata_removal` - Multiple file processing
- `test_metadata_preservation_after_copy` - Copy behavior verification

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Category

```bash
# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration_tests

# Metadata tests
cargo test --test metadata_tests

# Logger tests
cargo test --test logger_tests
```

### Run Single Test

```bash
cargo test test_has_metadata_with_exif_image
```

### Verbose Output

```bash
cargo test -- --nocapture
```

## Dependencies

Test-specific dependencies in `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3.14.0"  # Temporary directories for test isolation
```

## Test Data

Tests use images from `imgs/` directory:

- `imgs/note102.jpg` - Sample image with EXIF metadata
- `imgs/note102_medars.jpg` - Pre-cleaned image

## Graceful Degradation

Tests that require `rexiv2` (metadata removal) gracefully skip if:

- `libgexiv2` system library not installed
- File write operations fail
- Image format not supported

Error messages indicate when tests are skipped due to missing dependencies.

## CI/CD Integration

Tests are designed to:

- Run quickly (< 5 seconds total)
- Handle missing system libraries
- Provide clear failure messages
- Avoid flaky behavior with timeouts

## Coverage Areas

✅ **Covered:**

- CLI command parsing
- Metadata reading (EXIF, XMP, IPTC)
- Metadata display (table and JSON formats)
- Error handling and validation
- Logging operations
- File I/O operations
- Glob pattern matching

⚠️ **Partial Coverage:**

- TUI interface (requires interactive testing)
- Image preview functionality (terminal protocol dependent)
- Async image loading (tested via integration)

❌ **Not Covered:**

- Terminal UI interactions (requires manual testing)
- Keyboard event handling
- Image protocol negotiation
- Cross-platform terminal compatibility

## Known Limitations

1. **rexiv2 dependency**: Tests skip metadata removal if system library unavailable
2. **Test images**: Some tests skip if sample images missing
3. **TUI testing**: Interactive UI requires manual testing
4. **Image protocols**: Terminal image display not tested

## Future Improvements

- [ ] Add TUI automated tests using virtual terminal
- [ ] Mock rexiv2 for consistent test behavior
- [ ] Add benchmarking tests for large image batches
- [ ] Test memory usage with large metadata sets
- [ ] Add property-based tests with quickcheck
- [ ] CI/CD pipeline integration
- [ ] Code coverage reporting with tarpaulin

## Troubleshooting

### Test Failures with rexiv2

If seeing "Failed to save image without metadata":

```bash
# Ubuntu/Debian
sudo apt-get install libgexiv2-dev

# Arch Linux
sudo pacman -S libgexiv2
```

### Missing Test Images

If tests skip due to missing images:

```bash
# Ensure test images exist
ls imgs/note102.jpg
```

### Timeout Issues

Integration tests have 150s timeout. If hitting limits:

```bash
# Run with increased timeout
RUST_TEST_THREADS=1 cargo test -- --test-threads=1
```

## Contributing

When adding new features:

1. Add unit tests inline in source files
2. Add integration tests in `tests/` directory
3. Update this documentation
4. Ensure all tests pass before PR
5. Add test for both success and error cases
