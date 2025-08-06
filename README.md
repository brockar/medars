# MEDARS

**ME**ta**DA**ta from image files in **R**u**S**t - A fast and simple command-line tool for inspecting and removing metadata from image files.

---

## WIP

## Features

- **View metadata**: Display metadata in human-readable table or JSON format
- **Remove metadata**: Clean images by removing all embedded metadata
- **Interactive TUI**: Terminal user interface for easy navigation

## Privacy & Security

MEDARS helps protect your privacy by:

- Removing potentially sensitive EXIF data (GPS coordinates, camera settings, timestamps)
- Working locally - no data sent to external services
- Preserving image quality while removing metadata

## Dependencies

This project requires the `gexiv2` library and its development headers.

On Ubuntu/Debian:
    sudo apt install libgexiv2-dev
On Arch:
    yay -S libgexiv2

If you see an error about `gexiv2.pc` or `gexiv2` not found, make sure the library is installed and `PKG_CONFIG_PATH` is set correctly.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Uses [exif](https://crates.io/crates/exif) for metadata reading
- Terminal UI powered by [ratatui](https://crates.io/crates/ratatui)
