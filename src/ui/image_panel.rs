use ratatui::{prelude::*, widgets::*};
use ratatui_image::{StatefulImage, Resize};
use ratatui_image::protocol::StatefulProtocol;

#[derive(Clone, Copy, PartialEq)]
pub enum ImageLoadStatus {
    NotImage,
    Loading,
    Loaded,
    Failed,
}

/// Renders the right panel (image preview or file name) in the TUI.
pub fn render_image_panel(
    f: &mut Frame,
    area: Rect,
    file_name: &str,
    image_state: Option<&mut StatefulProtocol>,
    load_status: ImageLoadStatus,
    _file_path: Option<&str>,
) {
    use ratatui::prelude::Alignment;
    
    // If we have an image state, render it with proper constraints
    if let Some(state) = image_state {
        // Create a smaller area for the image with padding
        let image_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        };
        
        // Use Resize::Fit with a maximum size to keep images smaller
        let max_width = image_area.width.min(60); // Limit max width
        let max_height = image_area.height.min(30); // Limit max height
        
        let constrained_area = Rect {
            x: image_area.x,
            y: image_area.y,
            width: max_width,
            height: max_height,
        };
        
        let widget = StatefulImage::default().resize(Resize::Fit(None));
        f.render_stateful_widget(widget, constrained_area, state);
        return;
    }
    
    // Show appropriate message based on loading status
    let (message, style) = match load_status {
        ImageLoadStatus::Loading => ("Loading image...", Style::default().fg(Color::Yellow)),
        ImageLoadStatus::Failed => ("❌ Failed to load image\n\nPress 'r' to retry", Style::default().fg(Color::Red)),
        ImageLoadStatus::NotImage => (file_name, Style::default().fg(Color::White)),
        ImageLoadStatus::Loaded => {
            // This should not happen if we have image_state, but show a fallback
            ("📷 Image loaded but not displayed", Style::default().fg(Color::Blue))
        },
    };
    
    let file_name_widget = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(style)
        .wrap(Wrap { trim: false });
    let inner_area = Rect {
        x: area.x,
        y: area.y + 2,
        width: area.width,
        height: area.height.saturating_sub(2),
    };
    f.render_widget(file_name_widget, inner_area);
}
