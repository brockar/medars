use crate::metadata::MetadataHandler;
use crate::ui::fast_image_loader::FastImageLoader;
use crate::ui::image_utils::ImageUtils;
use ratatui::widgets::ListState;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashSet;
use std::time::Instant;
use tokio::sync::mpsc;

/// Load an image file and create a StatefulProtocol for ratatui_image
fn load_image_protocol_sync(
    file_path: &std::path::Path,
    picker: &Picker,
    terminal_width: Option<u16>,
    terminal_height: Option<u16>,
) -> Result<StatefulProtocol, Box<dyn std::error::Error + Send + Sync>> {
    // Larger preview size for better quality on non-Kitty terminals
    let max_preview_width = 1200;
    let max_preview_height = 800;

    // Determine target size based on terminal or use defaults
    let (target_width, target_height) =
        if let (Some(width), Some(height)) = (terminal_width, terminal_height) {
            let (terminal_target_width, terminal_target_height) =
                FastImageLoader::get_terminal_display_size(width, height);
            (
                terminal_target_width.min(max_preview_width),
                terminal_target_height.min(max_preview_height),
            )
        } else {
            (max_preview_width, max_preview_height)
        };

    // Load the image using FastImageLoader with size constraints
    let img = FastImageLoader::load_image_resized(file_path, target_width, target_height).or_else(
        |_| -> Result<image::DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
            // Fallback: load and resize manually
            let img = image::open(file_path)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let (orig_width, orig_height) = (img.width(), img.height());
            if orig_width > target_width || orig_height > target_height {
                Ok(img.resize(
                    target_width,
                    target_height,
                    image::imageops::FilterType::Triangle,
                ))
            } else {
                Ok(img)
            }
        },
    )?;

    let protocol = picker.new_resize_protocol(img);
    Ok(protocol)
}

/// Load an image with priority settings for faster reload of previously processed images
fn load_image_protocol_priority(
    file_path: &std::path::Path,
    picker: &Picker,
    terminal_width: Option<u16>,
    terminal_height: Option<u16>,
) -> Result<StatefulProtocol, Box<dyn std::error::Error + Send + Sync>> {
    let max_preview_width = 1000;
    let max_preview_height = 700;

    let (target_width, target_height) =
        if let (Some(width), Some(height)) = (terminal_width, terminal_height) {
            let (terminal_target_width, terminal_target_height) =
                FastImageLoader::get_terminal_display_size(width, height);
            (
                terminal_target_width.min(max_preview_width),
                terminal_target_height.min(max_preview_height),
            )
        } else {
            (max_preview_width, max_preview_height)
        };

    // Load the image using FastImageLoader with size constraints
    let img = FastImageLoader::load_image_resized(file_path, target_width, target_height).or_else(
        |_| -> Result<image::DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
            let img = image::open(file_path)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let (orig_width, orig_height) = (img.width(), img.height());
            if orig_width > target_width || orig_height > target_height {
                // Use CatmullRom for balance between speed and quality
                Ok(img.resize(
                    target_width,
                    target_height,
                    image::imageops::FilterType::CatmullRom,
                ))
            } else {
                Ok(img)
            }
        },
    )?;

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
    pub files_without_metadata: HashSet<String>,
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
    pub last_loaded_path: Option<String>,     // Remember the last successfully loaded image path

    // Image picker for loading images
    pub image_picker: Option<Picker>,

    pub terminal_width: Option<u16>,
    pub terminal_height: Option<u16>,

    pub selected_files: HashSet<String>,
    pub popup_message: Option<String>,
    pub popup_time: Option<Instant>,
    
    // File list state for scrolling
    pub file_list_state: ListState,
    
    // Initial directory to prevent navigating above it
    pub initial_dir: Option<std::path::PathBuf>,
}

impl App {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let picker = Picker::from_query_stdio().ok();
        App {
            image_utils: ImageUtils::new(),
            image_state: None,
            image_path: None,
            files: Vec::new(),
            files_without_metadata: HashSet::new(),
            selected: 0,
            previous_selected: usize::MAX,
            cached_metadata_text: String::new(),
            focused_panel: FocusedPanel::Left,
            mid_scroll: 0,
            running: true,
            image_load_receiver: receiver,
            image_load_sender: sender,
            loading_images: HashSet::new(),
            failed_images: HashSet::new(),
            loaded_images: HashSet::new(),
            last_frame_time: Instant::now(),
            pending_current_load: None,
            last_loaded_path: None,
            image_picker: picker,
            terminal_width: None,
            terminal_height: None,
            selected_files: HashSet::new(),
            popup_message: None,
            popup_time: None,
            file_list_state: ListState::default(),
            initial_dir: None,
        }
    }
    
    pub fn set_initial_dir(&mut self, dir: std::path::PathBuf) {
        self.initial_dir = Some(dir);
    }

    /// Process any pending image load events from background tasks
    pub fn process_image_load_events(&mut self) {
        while let Ok(event) = self.image_load_receiver.try_recv() {
            match event {
                ImageLoadEvent::LoadComplete {
                    file_path,
                    protocol,
                } => {
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
                }

                ImageLoadEvent::LoadError {
                    file_path,
                    error: _,
                } => {
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
                let actual_filename = self.get_actual_filename(selected_file);
                let file_path = dir.join(&actual_filename);

                // Skip metadata/image loading for directories
                if self.is_directory_entry(selected_file) {
                    self.cached_metadata_text = format!("Directory: {}", actual_filename);
                    self.image_path = None;
                    self.image_state = None;
                    self.previous_selected = self.selected;
                    self.mid_scroll = 0;
                    return;
                }

                // Update cached metadata text
                self.cached_metadata_text = self
                    .image_utils
                    .get_metadata_for_display(selected_file, &file_path);

                // Update image path
                let file_path_str = file_path.to_string_lossy().to_string();
                self.image_path = Some(file_path_str.clone());

                // Check if image needs to be loaded
                if self.is_image_file(&file_path) {
                    // Smart image state management: only clear if we're not navigating to a recently loaded image
                    let should_clear_state = self.last_loaded_path.as_ref() != Some(&file_path_str)
                        || !self.loaded_images.contains(&file_path_str);

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
                    else if !self.loading_images.contains(&file_path_str)
                        && !self.failed_images.contains(&file_path_str)
                    {
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
                } else {
                    // Not an image file - clear the image state
                    self.image_state = None;
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
                let display_name = &self.files[i];
                if self.is_directory_entry(display_name) {
                    continue;
                }
                let actual_filename = self.get_actual_filename(display_name);
                let file_path = dir.join(&actual_filename);
                if self.is_image_file(&file_path) {
                    let file_path_str = file_path.to_string_lossy().to_string();
                    // Only start loading if not already loaded, loading, or failed
                    if !self.loading_images.contains(&file_path_str)
                        && !self.failed_images.contains(&file_path_str)
                        && !self.loaded_images.contains(&file_path_str)
                        && self.loading_images.len() < max_concurrent_loads
                    {
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
            let current_files: HashSet<String> = self
                .files
                .iter()
                .map(|f| dir.join(f).to_string_lossy().to_string())
                .collect();
            self.loaded_images
                .retain(|path| current_files.contains(path));
        }
    }

    fn collect_files(dir: &std::path::Path) -> Vec<String> {
        match std::fs::read_dir(dir) {
            Ok(read_dir) => {
                let mut entries: Vec<String> = read_dir
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();
                        let name = path.file_name()?.to_string_lossy().to_string();
                        
                        // Skip hidden files/folders (starting with .)
                        if name.starts_with('.') {
                            return None;
                        }
                        
                        if path.is_dir() {
                            Some(format!("[DIR] {}", name))
                        } else if path.is_file() {
                            Some(name)
                        } else {
                            None
                        }
                    })
                    .collect();
                
                // Sort: directories first, then files
                entries.sort_by(|a, b| {
                    let a_is_dir = a.starts_with("[DIR] ");
                    let b_is_dir = b.starts_with("[DIR] ");
                    
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.cmp(b),
                    }
                });
                
                entries
            }
            Err(_) => Vec::new(),
        }
    }
    
    pub fn get_actual_filename(&self, display_name: &str) -> String {
        if display_name.starts_with("[DIR] ") {
            display_name.trim_start_matches("[DIR] ").to_string()
        } else {
            display_name.to_string()
        }
    }
    
    pub fn is_directory_entry(&self, display_name: &str) -> bool {
        display_name.starts_with("[DIR] ")
    }

    pub fn refresh_file_list(&mut self, dir: &std::path::Path) {
        self.refresh_file_list_with_reset(dir, false);
    }

    pub fn refresh_file_list_with_reset(&mut self, dir: &std::path::Path, reset_to_first: bool) {
        let current_selection = self.files.get(self.selected).cloned();
        self.files = Self::collect_files(dir);
        self.files_without_metadata.clear();

        for file in &self.files {
            let actual_name = self.get_actual_filename(file);
            let path = dir.join(&actual_name);
            if !self.is_directory_entry(file) && self.is_image_file(&path) {
                if let Ok(false) = self.image_utils.metadata_handler.has_metadata(&path) {
                    self.files_without_metadata.insert(file.clone());
                }
            }
        }

        if reset_to_first {
            // When navigating to a new directory, start at the first item
            self.selected = 0;
        } else if let Some(current) = current_selection {
            if let Some(idx) = self.files.iter().position(|f| f == &current) {
                self.selected = idx;
            } else if !self.files.is_empty() {
                self.selected = 0; // Default to first item instead of last
            } else {
                self.selected = 0;
            }
        } else if self.files.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.files.len() {
            self.selected = self.files.len().saturating_sub(1);
        }

        self.file_list_state.select(Some(self.selected));
        self.previous_selected = usize::MAX;
        self.image_utils.cached_metadata = None;
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

    /// Keyboard input - returns Option<PathBuf> if directory navigation is needed
    pub fn handle_input(
        &mut self,
        key: crossterm::event::KeyCode,
        max_scroll: u16,
        dir: &std::path::Path,
    ) -> Option<std::path::PathBuf> {
        // If popup is visible, any keypress dismisses it
        if self.should_show_popup() {
            self.popup_message = None;
            self.popup_time = None;
            return None;
        }

        match key {
            crossterm::event::KeyCode::Char('q') => {
                self.running = false;
                None
            }
            // Enter key to navigate into directories or go up with '..'
            crossterm::event::KeyCode::Enter if self.focused_panel == FocusedPanel::Left => {
                if let Some(selected_file) = self.files.get(self.selected) {
                    if self.is_directory_entry(selected_file) {
                        let actual_filename = self.get_actual_filename(selected_file);
                        return Some(dir.join(actual_filename));
                    }
                }
                None
            }
            // Escape to go to parent directory
            crossterm::event::KeyCode::Esc if self.focused_panel == FocusedPanel::Left => {
                // Don't go above the initial directory
                if let Some(ref initial_dir) = self.initial_dir {
                    // Canonicalize both paths for proper comparison
                    if let (Ok(current_canonical), Ok(initial_canonical)) = 
                        (dir.canonicalize(), initial_dir.canonicalize()) {
                        if current_canonical == initial_canonical {
                            return None; // Already at initial directory
                        }
                    }
                }
                
                if let Some(parent) = dir.parent() {
                    return Some(parent.to_path_buf());
                }
                None
            }
            // Panel focus switching
            crossterm::event::KeyCode::Right | crossterm::event::KeyCode::Char('l') => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::Left => FocusedPanel::Middle,
                    FocusedPanel::Middle => FocusedPanel::Left, // cycle back
                };
                None
            }
            crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Char('h') => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::Middle => FocusedPanel::Left,
                    FocusedPanel::Left => FocusedPanel::Middle, // cycle back
                };
                None
            }
            // Only allow up/down navigation when left
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j')
                if self.focused_panel == FocusedPanel::Left =>
            {
                if self.selected < self.files.len().saturating_sub(1) {
                    self.selected += 1;
                    self.file_list_state.select(Some(self.selected));
                }
                None
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k')
                if self.focused_panel == FocusedPanel::Left =>
            {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.file_list_state.select(Some(self.selected));
                }
                None
            }
            // Scroll metadata
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j')
                if self.focused_panel == FocusedPanel::Middle =>
            {
                if self.mid_scroll < max_scroll {
                    self.mid_scroll += 1;
                }
                None
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k')
                if self.focused_panel == FocusedPanel::Middle =>
            {
                if self.mid_scroll > 0 {
                    self.mid_scroll -= 1;
                }
                None
            }
            crossterm::event::KeyCode::Char(' ') if self.focused_panel == FocusedPanel::Left => {
                // Toggle selection of the currently highlighted file (not directories)
                if let Some(file) = self.files.get(self.selected) {
                    if !self.is_directory_entry(file) {
                        if self.selected_files.contains(file) {
                            self.selected_files.remove(file);
                        } else {
                            self.selected_files.insert(file.clone());
                        }
                    }
                }
                None
            }
            crossterm::event::KeyCode::Char('a') if self.focused_panel == FocusedPanel::Left => {
                self.select_all_files();
                None
            }
            crossterm::event::KeyCode::Char('d') => {
                // Delete metadata of selected files (only if files are selected)
                if !self.selected_files.is_empty() {
                    self.delete_metadata_of_selected_files(dir);
                }
                None
            }
            crossterm::event::KeyCode::Char('c') => {
                if !self.selected_files.is_empty() {
                    self.copy_metadata_of_selected_files(dir);
                }
                None
            }
            _ => None,
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
            })
            .await;

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
                load_image_protocol_priority(
                    &file_path,
                    &picker_clone,
                    terminal_width,
                    terminal_height,
                )
            })
            .await;

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

    fn select_all_files(&mut self) {
        if self.files.is_empty() {
            return;
        }

        // Only select actual files, not directories
        let selectable_files: Vec<String> = self
            .files
            .iter()
            .filter(|f| !self.is_directory_entry(f))
            .cloned()
            .collect();

        // If all selectable files are already selected, deselect all. Otherwise, select all.
        if !selectable_files.is_empty() && self.selected_files.len() == selectable_files.len() {
            self.selected_files.clear();
        } else {
            self.selected_files = selectable_files.into_iter().collect();
        }
    }

    pub fn delete_metadata_of_selected_files(&mut self, dir: &std::path::Path) {
        let mut cleaned_files = Vec::new();
        let mut failed_files = Vec::new();

        for file in &self.selected_files {
            let file_path = dir.join(file);
            let output_path = file_path.clone(); // Overwrite the original file
            match self
                .image_utils
                .metadata_handler
                .remove_metadata(&file_path, &output_path)
            {
                Ok(_) => {
                    cleaned_files.push(file.clone());
                }
                Err(e) => {
                    failed_files.push((file.clone(), format!("{}", e)));
                }
            }
        }

        if !cleaned_files.is_empty() {
            self.refresh_file_list(dir);
            // After cleaning, force an update of the metadata panel for the current selection
            if !self.files.is_empty() && self.selected < self.files.len() {
                let selected_file = &self.files[self.selected];
                let file_path = dir.join(selected_file);
                self.cached_metadata_text = self
                    .image_utils
                    .get_metadata_for_display(selected_file, &file_path);
            }
        }

        // Build popup message
        let mut message = String::new();
        if !cleaned_files.is_empty() {
            message.push_str(&format!("✅ Cleaned {} file(s):\n", cleaned_files.len()));
            for file in &cleaned_files {
                message.push_str(&format!("  • {}\n", file));
            }
        }
        if !failed_files.is_empty() {
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str(&format!("❌ Failed {} file(s):\n", failed_files.len()));
            for (file, error) in &failed_files {
                message.push_str(&format!("  • {}: {}\n", file, error));
            }
        }

        self.popup_message = Some(message);
        self.popup_time = Some(Instant::now());
        self.selected_files.clear(); // Clear selection after processing
    }

    pub fn copy_metadata_of_selected_files(&mut self, dir: &std::path::Path) {
        let handler = MetadataHandler::new();
        let mut cleaned_files = Vec::new();
        let mut failed_files = Vec::new();

        for file in &self.selected_files {
            let file_path = dir.join(file);
            if !file_path.exists() {
                failed_files.push((file.clone(), "File does not exist".to_string()));
                continue;
            }

            let output_path = Self::derive_copy_output_path(&file_path);
            if let Some(parent) = output_path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        failed_files.push((
                            file.clone(),
                            format!("Failed to create output directory: {}", e),
                        ));
                        continue;
                    }
                }
            }

            if output_path != file_path {
                if let Err(e) = std::fs::copy(&file_path, &output_path) {
                    failed_files.push((file.clone(), format!("Failed to copy file: {}", e)));
                    continue;
                }
            }

            match handler.remove_metadata(&file_path, &output_path) {
                Ok(_) => {
                    let output_name = output_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| output_path.to_string_lossy().into_owned());
                    cleaned_files.push((file.clone(), output_name));
                }
                Err(e) => {
                    failed_files.push((file.clone(), format!("{}", e)));
                }
            }
        }

        self.refresh_file_list(dir);

        let mut message = String::new();
        if !cleaned_files.is_empty() {
            message.push_str(&format!("✅ Copied {} file(s):\n", cleaned_files.len()));
            for (original, output) in &cleaned_files {
                message.push_str(&format!("  • {} -> {}\n", original, output));
            }
        }
        if !failed_files.is_empty() {
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str(&format!("❌ Failed {} file(s):\n", failed_files.len()));
            for (file, error) in &failed_files {
                message.push_str(&format!("  • {}: {}\n", file, error));
            }
        }

        self.popup_message = Some(message);
        self.popup_time = Some(Instant::now());
        self.selected_files.clear();
    }

    fn derive_copy_output_path(file_path: &std::path::Path) -> std::path::PathBuf {
        let parent = file_path.parent();
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let mut new_name = format!("{}_medars", stem);
        if !ext.is_empty() {
            new_name.push('.');
            new_name.push_str(ext);
        }

        match parent {
            Some(p) if !p.as_os_str().is_empty() => p.join(new_name),
            _ => std::path::PathBuf::from(new_name),
        }
    }

    /// Check if popup should still be displayed (show for 3 seconds)
    pub fn should_show_popup(&self) -> bool {
        if let Some(popup_time) = self.popup_time {
            popup_time.elapsed().as_secs() < 3
        } else {
            false
        }
    }

    /// Clear popup if time has expired
    pub fn clear_expired_popup(&mut self) {
        if self.popup_time.is_some() && !self.should_show_popup() {
            self.popup_message = None;
            self.popup_time = None;
        }
    }
}
