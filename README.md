# MEDARS

**ME**ta**DA**ta from image files in **R**u**S**t - A fast and simple command-line tool for inspecting and removing metadata from image files.

---

## Features

- **Check Metadata**: Check if an image contains metadata.
- **View Metadata**: Display metadata in a human-readable table or JSON format.
- **Remove Metadata**: Clean images by removing all embedded metadata.
- **Interactive TUI**: Terminal user interface for easy navigation and image preview.
- **Log Actions**: Keep a log of all operations performed.

## Core Functionality

### CLI mode

- **Check for metadata:**

  ```bash
  medars check image.jpg
  ```

- **Show metadata:**

  ```bash
  medars show image.jpg
  ```

- **Clean metadata:**

  ```bash
  medars clean image.jpg
  ```

- **Launch the TUI:**

  ```bash
  medars tui
  ```

  or

  ```bash
  medars tui <path/to/directory>
  ```

- **Batch operations:**

  ```bash
  medars clean "*.jpg"
  medars clean path1.jpg path2.png
  ```

- **Flags:**
  - `--copy [PATH]` → Save as a new file. If `PATH` is not provided, it will be saved with a `_medars` suffix.
  - `--dry-run` → Show what will be removed without modifying the file.

## Privacy & Security

MEDARS helps protect your privacy by:

- Removing potentially sensitive EXIF data (GPS coordinates, camera settings, timestamps).
- Working locally - no data is sent to external services.
- Preserving image quality while removing metadata.

## Dependencies

This project requires the `gexiv2` library and its development headers.

On Ubuntu/Debian:

```bash
sudo apt install libgexiv2-2
```

On Arch:

```bash
yay -S libgexiv2
```

If you see an error about `gexiv2.pc` or `gexiv2` not found, make sure the library is installed.

## Installation

### From Crates.io (once published)

```sh
cargo install medars
```

### From Git Repository

```sh
cargo install --git https://github.com/your-username/medars.git
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/).
- Uses [rexiv2](https://crates.io/crates/rexiv2) and [kamadak-exif](https://crates.io/crates/kamadak-exif) for metadata handling.
- CLI powered by [clap](https://crates.io/crates/clap).
- Terminal UI powered by [ratatui](https://crates.io/crates/ratatui).
