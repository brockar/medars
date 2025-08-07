use ratatui::{prelude::*, widgets::*};

/// Renders the right panel (image preview or file name) in the TUI.
pub fn render_image_panel(
    f: &mut Frame,
    area: Rect,
    file_name: &str,
) {
    // Show the file name centered, a few lines down from the top border.
    use ratatui::prelude::Alignment;
    let file_name_widget = Paragraph::new(file_name)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    // Offset the area downward to avoid overlapping the panel title
    let inner_area = Rect {
        x: area.x,
        y: area.y + 2, // 2 lines below the top border/title
        width: area.width,
        height: area.height.saturating_sub(2),
    };
    f.render_widget(file_name_widget, inner_area);
}
