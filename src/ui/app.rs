use crate::ui::image_utils::ImageUtils;
use crate::ui::fast_image_loader::FastImageLoader;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::picker::Picker;
use tokio::sync::mpsc;
use std::collections::HashSet;
use std::time::Instant;

/// Load an image file and create a StatefulProtocol for ratatui_image
fn load_image_protocol_sync(
    file_path: &std::path::Path, 
    picker: &Picker,
    terminal_width: Option<u16>,
    terminal_height: Option<u16>
) -> Result<StatefulProtocol, Box<dyn std::error::Error + Send + Sync>> {
    // Down scale the image to faster preview
    let max_preview_width = 600;  
    let max_preview_height = 400; 
    
    // Determine target size based on terminal or use defaults
    let (target_width, target_height) = if let (Some(width), Some(height)) = (terminal_width, terminal_height) {
        let (terminal_target_width, terminal_target_height) = FastImageLoader::get_terminal_display_size(width, height);
        (terminal_target_width.min(max_preview_width), terminal_target_height.min(max_preview_height))
    } else {
        (max_preview_width, max_preview_height)
    };

    // Load the image using FastImageLoader with size constraints
    let img = FastImageLoader::load_image_resized(file_path, target_width, target_height)
        .or_else(|_| -> Result<image::DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
            // Fallback: load and resize manually
            let img = image::open(file_path).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let (orig_width, orig_height) = (img.width(), img.height());
            if orig_width > target_width || orig_height > target_height {
                Ok(img.resize(target_width, target_height, image::imageops::FilterType::Triangle))
            } else {
                Ok(img)
            }
        })?;

    let protocol = picker.new_resize_protocol(img);
    Ok(protocol)
}

/// Load an image with priority settings for faster reload of previously processed images
fn load_image_protocol_priority(
    file_path: &std::path::Path, 
    picker: &Picker,
    terminal_width: Option<u16>,
    terminal_height: Option<u16>
) -> Result<StatefulProtocol, Box<dyn std::error::Error + Send + Sync>> {
    let max_preview_width = 500;  
    let max_preview_height = 350; 
    
    let (target_width, target_height) = if let (Some(width), Some(height)) = (terminal_width, terminal_height) {
        let (terminal_target_width, terminal_target_height) = FastImageLoader::get_terminal_display_size(width, height);
        (terminal_target_width.min(max_preview_width), terminal_target_height.min(max_preview_height))
    } else {
        (max_preview_width, max_preview_height)
    };

    // Load the image using FastImageLoader with very aggressive size constraints
    let img = FastImageLoader::load_image_resized(file_path, target_width, target_height)
        .or_else(|_| -> Result<image::DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
            // For priority loads, skip the file size check and try to load anyway
            let img = image::open(file_path).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let (orig_width, orig_height) = (img.width(), img.height());
            if orig_width > target_width || orig_height > target_height {
                // Use fastest resize filter for priority loads
                Ok(img.resize(target_width, target_height, image::imageops::FilterType::Nearest))
            } else {
                Ok(img)
            }
        })?;

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
        #[allow(dead_code)]
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
    pub loaded_images: HashSet<String>, // Track successfully loaded images to avoid reloading
    pub last_frame_time: Instant,
    pub pending_current_load: Option<String>, // Track if it's waiting for current selection to load
    pub last_loaded_path: Option<String>, // Remember the last successfully loaded image path

    // Image picker for loading images
    pub image_picker: Option<Picker>,

    pub terminal_width: Option<u16>,
    pub terminal_height: Option<u16>,
}

impl App {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        // Try to initialize the image picker once during app creation
        let picker = Picker::from_query_stdio().ok();
        if picker.is_none() {
            eprintln!("Note: Image preview not available in this terminal. Use a terminal with image support (Kitty, WezTerm, or Ghostty) for full functionality.");
        }
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
            loaded_images: HashSet::new(), // Track successfully loaded images
            last_frame_time: Instant::now(),
            pending_current_load: None, // No pending loads initially
            last_loaded_path: None, // No previously loaded image
            image_picker: picker,
            terminal_width: None,
            terminal_height: None,
        }
    }

    /// Process any pending image load events from background tasks
    pub fn process_image_load_events(&mut self) {
        while let Ok(event) = self.image_load_receiver.try_recv() {
            match event {
                ImageLoadEvent::LoadComplete { file_path, protocol } => {
                    // Mark as successfully loaded
                    self.loaded_images.insert(file_path.clone());
                    self.last_loaded_path = Some(file_path.clone()); // Remember this image
                    
                    // Always update image state if this is for the currently selected image
                    if let Some(ref current_path) = self.image_path {
                        if current_path == &file_path {
                            self.image_state = Some(protocol);
                            self.pending_current_load = None; // Clear pending flag
                        }
                    }
                    // Always remove from loading set 
                    self.loading_images.remove(&file_path);
                },

                ImageLoadEvent::LoadError { file_path, error: _ } => {
                    // Mark as failed and remove from loading
                    self.failed_images.insert(file_path.clone());
                    self.loading_images.remove(&file_path);
                    
                    // Clear pending flag if this was the current selection
                    if let Some(ref current_path) = self.image_path {
                        if current_path == &file_path {
                            self.pending_current_load = None;
                        }
                    }
                }
            }
        }
    }

    /// Update terminal dimensions for image loading
    pub fn update_terminal_size(&mut self, width: u16, height: u16) {
        self.terminal_width = Some(width);
        self.terminal_height = Some(height);
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
                
                // Check if image needs to be loaded
                if self.is_image_file(&file_path) {
                    // Smart image state management: only clear if we're not navigating to a recently loaded image
                    let should_clear_state = self.last_loaded_path.as_ref() != Some(&file_path_str) || 
                                           !self.loaded_images.contains(&file_path_str);
                    
                    if should_clear_state {
                        self.image_state = None; 
                    }
                    
                    // Check if we already have this image loaded, prioritize it for fast reload
                    if self.loaded_images.contains(&file_path_str) {
                        // Image was previously loaded
                        if !self.loading_images.contains(&file_path_str) {
                            self.pending_current_load = Some(file_path_str.clone());
                            self.start_priority_image_load(file_path); 
                        }
                    }
                    // For new images, use normal loading
                    else if !self.loading_images.contains(&file_path_str) && 
                       !self.failed_images.contains(&file_path_str) {
                        self.pending_current_load = Some(file_path_str.clone());
                        self.start_background_image_load(file_path);
                    } 
                    // Retry failed images 
                    else if self.failed_images.contains(&file_path_str) {
                        self.failed_images.remove(&file_path_str);
                        if !self.loading_images.contains(&file_path_str) {
                            self.pending_current_load = Some(file_path_str.clone());
                            self.start_background_image_load(file_path);
                        }
                    }
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

        // Don't preload if we're still waiting for the current selection to load
        if self.pending_current_load.is_some() {
            return;
        }

        let preload_range: usize = 2;
        let start = self.selected.saturating_sub(preload_range);
        let end = (self.selected + preload_range + 1).min(self.files.len());

        let max_concurrent_loads = 2;
        if self.loading_images.len() >= max_concurrent_loads {
            return;
        }

        for i in start..end {
            if i != self.selected {
                let file_path = dir.join(&self.files[i]);
                if self.is_image_file(&file_path) {
                    let file_path_str = file_path.to_string_lossy().to_string();
                    // Only start loading if not already loaded, loading, or failed
                    if !self.loading_images.contains(&file_path_str) && 
                       !self.failed_images.contains(&file_path_str) &&
                       !self.loaded_images.contains(&file_path_str) &&
                       self.loading_images.len() < max_concurrent_loads {
                        self.start_background_image_load(file_path);
                        break; 
                    }
                }
            }
        }

        // Clean up old tracking to prevent memory bloat
        // Reset failure tracking occasionally to allow retries
        if self.failed_images.len() > 20 {
            self.failed_images.clear();
        }
        
        // Keep loaded tracking reasonable size
        if self.loaded_images.len() > 50 {
            let current_files: HashSet<String> = self.files.iter()
                .map(|f| dir.join(f).to_string_lossy().to_string())
                .collect();
            self.loaded_images.retain(|path| current_files.contains(path));
        }
    }

    /// Get the loading status for the currently selected image
    pub fn get_image_load_status(&self) -> crate::ui::image_panel::ImageLoadStatus {
        if let Some(ref current_path) = self.image_path {
            let path = std::path::Path::new(current_path);
            let is_image = self.is_image_file(path);
            
            if !is_image {
                crate::ui::image_panel::ImageLoadStatus::NotImage
            } else if self.image_picker.is_none() {
                // Terminal doesn't support image rendering
                crate::ui::image_panel::ImageLoadStatus::UnsupportedTerminal
            } else if self.loading_images.contains(current_path) {
                crate::ui::image_panel::ImageLoadStatus::Loading
            } else if self.failed_images.contains(current_path) {
                crate::ui::image_panel::ImageLoadStatus::Failed
            } else if self.image_state.is_some() {
                crate::ui::image_panel::ImageLoadStatus::Loaded
            } else {
                // Image file but not loaded yet, startloading
                crate::ui::image_panel::ImageLoadStatus::Loading
            }
        } else {
            crate::ui::image_panel::ImageLoadStatus::NotImage
        }
    }

    /// Keyboard input
    pub fn handle_input(&mut self, key: crossterm::event::KeyCode, max_scroll: u16, _dir: &std::path::Path) {
        match key {
            crossterm::event::KeyCode::Char('q') => self.running = false,
            // Panel focus switching
            crossterm::event::KeyCode::Right | crossterm::event::KeyCode::Char('l') => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::Left => FocusedPanel::Middle,
                    FocusedPanel::Middle => FocusedPanel::Left, // cycle back
                };
            }
            crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Char('h') => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::Middle => FocusedPanel::Left,
                    FocusedPanel::Left => FocusedPanel::Middle, // cycle back
                };
            }
            // Only allow up/down navigation when left
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
            // Scroll metadata
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
        
        // Don't load if already loading
        if self.loading_images.contains(&file_path_str) {
            return;
        }

        // Don't load if we don't have a picker
        let Some(picker) = self.image_picker.as_ref() else {
            return;
        };

        // Clear from failed/loaded state to allow fresh load
        self.failed_images.remove(&file_path_str);
        self.loaded_images.remove(&file_path_str);
        
        self.loading_images.insert(file_path_str.clone());
        
        let sender = self.image_load_sender.clone();
        let picker_clone = picker.clone();
        let terminal_width = self.terminal_width;
        let terminal_height = self.terminal_height;
        tokio::spawn(async move {
            // Try to load the image using ratatui_image
            let result = tokio::task::spawn_blocking(move || {
                load_image_protocol_sync(&file_path, &picker_clone, terminal_width, terminal_height)
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
    /// Start loading an image with high priority (for previously loaded images)
    fn start_priority_image_load(&mut self, file_path: std::path::PathBuf) {
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Don't load if already loading
        if self.loading_images.contains(&file_path_str) {
            return;
        }
        // Don't load if we don't have a picker
        let Some(picker) = self.image_picker.as_ref() else {
            return;
        };
        // Don't clear from loaded_images for priority loads - keep the cache
        // Only clear from failed state
        self.failed_images.remove(&file_path_str);
        self.loading_images.insert(file_path_str.clone());
        
        let sender = self.image_load_sender.clone();
        let picker_clone = picker.clone();
        let terminal_width = self.terminal_width;
        let terminal_height = self.terminal_height;
        
        // Use a higher priority task for previously loaded images
        tokio::spawn(async move {
            // For priority loads, use even smaller sizes for faster processing
            let result = tokio::task::spawn_blocking(move || {
                load_image_protocol_priority(&file_path, &picker_clone, terminal_width, terminal_height)
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
}
