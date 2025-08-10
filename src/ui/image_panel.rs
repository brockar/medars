use ratatui::{prelude::*, widgets::*};
use ratatui_image::{StatefulImage, Resize};
use ratatui_image::protocol::StatefulProtocol;

#[derive(Clone, Copy, PartialEq)]
pub enum ImageLoadStatus {
    NotImage,
    Loading,
    Loaded,
    Failed,
    UnsupportedTerminal,
}

pub fn render_image_panel(
    f: &mut Frame,
    area: Rect,
    file_name: &str,
    image_state: Option<&mut StatefulProtocol>,
    load_status: ImageLoadStatus,
    _file_path: Option<&str>,
) {
    use ratatui::prelude::Alignment;
    
    if let Some(state) = image_state {
        let available_area = Rect {
            x: area.x + 1,
            y: area.y + 2, 
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };
        
        let margin_x = 2;
        let margin_y = 1;
        
        let centered_area = Rect {
            x: available_area.x + margin_x,
            y: available_area.y + margin_y,
            width: available_area.width.saturating_sub(margin_x * 2),
            height: available_area.height.saturating_sub(margin_y * 2),
        };
        
        // Use Resize::Fit which should center the image within the given area
        // while maintaining aspect ratio
        let widget = StatefulImage::default().resize(Resize::Fit(None));
        f.render_stateful_widget(widget, centered_area, state);
        return;
    }
    
    // Show appropriate message based on loading status
    let (message, style) = match load_status {
        ImageLoadStatus::Loading => ("Loading image...", Style::default().fg(Color::Yellow)),
        ImageLoadStatus::Failed => ("❌ Failed to load image\n", Style::default().fg(Color::Red)),
        ImageLoadStatus::NotImage => (file_name, Style::default().fg(Color::White)),
        ImageLoadStatus::UnsupportedTerminal => {
            ("📷 Image Preview Unavailable\n\nTerminal doesn't support image rendering.\nTry using Kitty, iTerm2, or a terminal\nwith Sixel support for image preview.\n\nMetadata is shown in the left panel.", 
             Style::default().fg(Color::Cyan))
        },
        // This doesn't hhappends but have to have the option (?)
        ImageLoadStatus::Loaded => {
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
