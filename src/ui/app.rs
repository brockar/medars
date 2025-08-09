use crate::ui::image_utils::ImageUtils;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::picker::Picker;
use tokio::sync::mpsc;
use std::collections::HashSet;
use std::time::Instant;

/// Load an image file and create a StatefulProtocol for ratatui_image
fn load_image_protocol_sync(file_path: &std::path::Path, picker: &Picker) -> Result<StatefulProtocol, Box<dyn std::error::Error + Send + Sync>> {
    // Load the image using the image crate
    let img = image::open(file_path)?;
    
    // Create the protocol using the provided picker
    let protocol = picker.new_resize_protocol(img);
    
    Ok(protocol)
}

#[derive(Copy, Clone, PartialEq)]
pub enum FocusedPanel {
    Left,
    Middle,
}

pub enum ImageLoadEvent {
    LoadComplete {
        file_path: String,
        protocol: StatefulProtocol,
    },
    LoadError {
        file_path: String,
        error: String,
    },
}

/// Central application state struct holding all UI state
pub struct App {
    pub image_utils: ImageUtils,
    pub image_state: Option<StatefulProtocol>,
    pub image_path: Option<String>,
    pub files: Vec<String>,
    pub selected: usize,
    pub previous_selected: usize,
    pub cached_metadata_text: String,
    pub focused_panel: FocusedPanel,
    pub mid_scroll: u16,
    pub running: bool,
    // Background loading infrastructure
    pub image_load_receiver: mpsc::UnboundedReceiver<ImageLoadEvent>,
    pub image_load_sender: mpsc::UnboundedSender<ImageLoadEvent>,
    pub loading_images: HashSet<String>,
    pub failed_images: HashSet<String>,
    pub last_frame_time: Instant,
    // Image picker for loading images
    pub image_picker: Option<Picker>,
}

impl App {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        // Try to initialize the image picker once during app creation
        let picker = Picker::from_query_stdio().ok();
        App {
            image_utils: ImageUtils::new(),
            image_state: None,
            image_path: None,
            files: Vec::new(),
            selected: 0,
            previous_selected: usize::MAX, // Force initial load
            cached_metadata_text: String::new(),
            focused_panel: FocusedPanel::Left,
            mid_scroll: 0,
            running: true,
            image_load_receiver: receiver,
            image_load_sender: sender,
            loading_images: HashSet::new(),
            failed_images: HashSet::new(), // Start with clean state
            last_frame_time: Instant::now(),
            image_picker: picker,
        }
    }

    /// Process any pending image load events from background tasks
    pub fn process_image_load_events(&mut self) {
        while let Ok(event) = self.image_load_receiver.try_recv() {
            match event {
                ImageLoadEvent::LoadComplete { file_path, protocol } => {
                    self.loading_images.remove(&file_path);
                    if let Some(ref current_path) = self.image_path {
                        if *current_path == file_path {
                            self.image_state = Some(protocol);
                        }
                    }
                }
                ImageLoadEvent::LoadError { file_path, error: _ } => {
                    self.loading_images.remove(&file_path);
                    self.failed_images.insert(file_path);
                }
            }
        }
    }

    /// Update selection and load metadata/image for the selected file
    pub fn update_selection(&mut self, dir: &std::path::Path) {
        if self.selected != self.previous_selected {
            if !self.files.is_empty() && self.selected < self.files.len() {
                let selected_file = &self.files[self.selected];
                let file_path = dir.join(selected_file);
                
                // Update cached metadata text
                self.cached_metadata_text = self.image_utils.get_metadata_for_display(selected_file, &file_path);
                
                // Update image path
                let file_path_str = file_path.to_string_lossy().to_string();
                self.image_path = Some(file_path_str.clone());
                
                // Clear previous image state
                self.image_state = None;
                
                // Start loading image if it's an image file
                if self.is_image_file(&file_path) {
                    self.start_background_image_load(file_path);
                }
            } else {
                self.cached_metadata_text = "No files available".to_string();
                self.image_path = None;
                self.image_state = None;
            }
            self.previous_selected = self.selected;
            self.mid_scroll = 0;
        }
    }

    /// Preload images for files around the current selection for smoother navigation
    pub fn preload_nearby_images(&mut self, dir: &std::path::Path) {
        if self.files.is_empty() {
            return;
        }

        // Preload 2 images before and after current selection
        let preload_range = 2;
        let start = self.selected.saturating_sub(preload_range);
        let end = (self.selected + preload_range + 1).min(self.files.len());

        for i in start..end {
            if i != self.selected {
                let file_path = dir.join(&self.files[i]);
                if self.is_image_file(&file_path) {
                    let file_path_str = file_path.to_string_lossy().to_string();
                    if !self.loading_images.contains(&file_path_str) && !self.failed_images.contains(&file_path_str) {
                        self.start_background_image_load(file_path);
                    }
                }
            }
        }
    }

    /// Get the loading status for the currently selected image
    pub fn get_image_load_status(&self) -> crate::ui::image_panel::ImageLoadStatus {
        if let Some(ref current_path) = self.image_path {
            // Check if it's actually an image
            let path = std::path::Path::new(current_path);
            let is_image = self.is_image_file(path);
            
            if !is_image {
                crate::ui::image_panel::ImageLoadStatus::NotImage
            } else if self.loading_images.contains(current_path) {
                crate::ui::image_panel::ImageLoadStatus::Loading
            } else if self.failed_images.contains(current_path) {
                crate::ui::image_panel::ImageLoadStatus::Failed
            } else if self.image_state.is_some() {
                crate::ui::image_panel::ImageLoadStatus::Loaded
            } else {
                // Image file but not loaded yet - should start loading
                crate::ui::image_panel::ImageLoadStatus::Loading
            }
        } else {
            crate::ui::image_panel::ImageLoadStatus::NotImage
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: crossterm::event::KeyCode, max_scroll: u16, dir: &std::path::Path) {
        match key {
            crossterm::event::KeyCode::Char('q') => self.running = false,
            // Retry failed image with 'r' key
            crossterm::event::KeyCode::Char('r') => {
                self.retry_failed_image(dir);
            }
            // Panel focus switching
            crossterm::event::KeyCode::Right | crossterm::event::KeyCode::Char('l') => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::Left => FocusedPanel::Middle,
                    FocusedPanel::Middle => FocusedPanel::Left, // cycle back for now
                };
            }
            crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Char('h') => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::Middle => FocusedPanel::Left,
                    FocusedPanel::Left => FocusedPanel::Middle, // cycle back for now
                };
            }
            // Only allow up/down navigation when left panel is focused
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') if self.focused_panel == FocusedPanel::Left => {
                if self.selected < self.files.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') if self.focused_panel == FocusedPanel::Left => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            // Scroll metadata when middle panel is focused
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') if self.focused_panel == FocusedPanel::Middle => {
                if self.mid_scroll < max_scroll {
                    self.mid_scroll += 1;
                }
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') if self.focused_panel == FocusedPanel::Middle => {
                if self.mid_scroll > 0 {
                    self.mid_scroll -= 1;
                }
            }
            _ => {}
        }
    }

    /// Check if a file is an image based on its extension
    fn is_image_file(&self, path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension() {
            match ext.to_string_lossy().to_lowercase().as_str() {
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" => true,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Start loading an image in the background
    fn start_background_image_load(&mut self, file_path: std::path::PathBuf) {
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Don't load if already loading, but allow retry of failed images
        if self.loading_images.contains(&file_path_str) {
            return;
        }

        // Don't load if we don't have a picker
        let Some(picker) = self.image_picker.as_ref() else {
            return;
        };

        // Remove from failed images to allow retry
        self.failed_images.remove(&file_path_str);
        
        self.loading_images.insert(file_path_str.clone());
        
        let sender = self.image_load_sender.clone();
        let picker_clone = picker.clone();
        tokio::spawn(async move {
            // Try to load the image using ratatui_image
            let result = tokio::task::spawn_blocking(move || {
                load_image_protocol_sync(&file_path, &picker_clone)
            }).await;
            
            match result {
                Ok(Ok(protocol)) => {
                    let _ = sender.send(ImageLoadEvent::LoadComplete {
                        file_path: file_path_str,
                        protocol,
                    });
                }
                Ok(Err(e)) => {
                    let _ = sender.send(ImageLoadEvent::LoadError {
                        file_path: file_path_str,
                        error: format!("Failed to load image: {}", e),
                    });
                }
                Err(e) => {
                    let _ = sender.send(ImageLoadEvent::LoadError {
                        file_path: file_path_str,
                        error: format!("Task failed: {}", e),
                    });
                }
            }
        });
    }

    /// Clear failure state for an image and retry loading it
    fn retry_failed_image(&mut self, _dir: &std::path::Path) {
        if let Some(ref current_path) = self.image_path.clone() {
            if self.failed_images.contains(current_path) {
                self.failed_images.remove(current_path);
                let path = std::path::Path::new(current_path);
                if self.is_image_file(path) {
                    self.start_background_image_load(path.to_path_buf());
                }
            }
        }
    }
}
