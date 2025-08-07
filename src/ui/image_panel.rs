use ratatui::{prelude::*, widgets::*};

/// Renders the right panel (image preview or file name) in the TUI.
pub fn render_image_panel(
    f: &mut Frame,
    area: Rect,
    file_name: &str,
) {
    // For now, just show the file name. Replace with image rendering later.
    let name_widget = Paragraph::new(file_name)
        .block(Block::default().title("File Name").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(name_widget, area);
}
