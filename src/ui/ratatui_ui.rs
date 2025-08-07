#[derive(Copy, Clone, PartialEq)]
enum FocusedPanel {
    Left,
    Middle,
}
use std::path::PathBuf;
use anyhow::Result;
use crate::ui::image_panel::render_image_panel;
use crate::ui::image_utils::ImageUtils;

pub struct RatatuiUI {
    image_utils: ImageUtils,
}


impl RatatuiUI {
    pub fn new() -> Self {
        RatatuiUI {
            image_utils: ImageUtils::new(),
        }
    }

    pub async fn run(&mut self, file: Option<PathBuf>) -> Result<()> {
        use ratatui::{prelude::*, widgets::*, layout::{Layout, Constraint, Direction}};
        use crossterm::{terminal, ExecutableCommand};
        use std::io::stdout;
        use tokio::task;
        use std::fs;
        use crossterm::event::{self, Event, KeyCode};

        let mut stdout = stdout();
        terminal::enable_raw_mode()?;
        stdout.execute(terminal::EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend) {
            Ok(t) => t,
            Err(e) => {
                let _ = terminal::disable_raw_mode();
                let _ = std::io::stdout().execute(terminal::LeaveAlternateScreen);
                return Err(e.into());
            }
        };

        // Branch: if file is Some and is a file, show image preview; else show file browser
        let mut running = true;
        if let Some(ref path) = file {
            if path.is_file() {
                // Show a centered placeholder text for a single file
                while running {
                    terminal.draw(|f| {
                        let area = f.area();
                        let block = Block::default().title("medars").borders(Borders::ALL);
                        let placeholder = Paragraph::new("[medars] File mode: UI placeholder\n(Feature coming soon)")
                            .block(block)
                            .alignment(Alignment::Center)
                            .wrap(Wrap { trim: true });
                        f.render_widget(placeholder, area);
                    })?;

                    let poll_res = task::spawn_blocking(|| event::poll(std::time::Duration::from_millis(200))).await;
                    if let Ok(Ok(true)) = poll_res {
                        let read_res = task::spawn_blocking(|| event::read()).await;
                        if let Ok(Ok(Event::Key(key))) = read_res {
                            match key.code {
                                KeyCode::Char('q') => running = false,
                                _ => {}
                            }
                        }
                    }
                }
                let _ = terminal::disable_raw_mode();
                let _ = std::io::stdout().execute(terminal::LeaveAlternateScreen);
                return Ok(());
            }
        }

        // Directory or no file: show file browser UI (original)
        // List files in current dir or given dir
        let dir: &std::path::Path = match file.as_ref() {
            Some(p) if p.is_dir() => p.as_path(),
            Some(p) => p.parent().unwrap_or(std::path::Path::new(".")),
            None => std::path::Path::new("."),
        };
        let files: Vec<String> = match fs::read_dir(dir) {
            Ok(read_dir) => read_dir.filter_map(|e| {
                let e = e.ok()?;
                let path = e.path();
                if path.is_file() {
                    path.file_name().map(|n| n.to_string_lossy().to_string())
                } else {
                    None
                }
            }).collect(),
            Err(_) => vec![],
        };

        let mut selected = 0;
        let mut previous_selected = usize::MAX; // Force initial load
        let mut cached_metadata_text = String::new();
        let mut focused_panel = FocusedPanel::Left;
        let mut mid_scroll: u16 = 0;

        while running {
            // Update metadata cache only when selection changes
            if selected != previous_selected {
                if !files.is_empty() {
                    let selected_file = &files[selected];
                    let file_path = dir.join(selected_file);
                    cached_metadata_text = self.image_utils.get_metadata_for_display(selected_file, &file_path);
                } else {
                    cached_metadata_text = "No files found".to_string();
                }
                previous_selected = selected;
                mid_scroll = 0;
            }


            // Calculate visible height for metadata panel (minus borders and title)
            let mut visible_height = 0u16;
            let mut max_scroll = 0u16;
            let mut total_lines = 0u16;
            terminal.draw(|f| {
                let area = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([
                        Constraint::Percentage(25), // File browser
                        Constraint::Percentage(40), // Metadata
                        Constraint::Percentage(35), // Image preview
                    ])
                    .split(area);

                // Count display lines, including wrapped/multiline JSON
                let count_display_lines = |text: &str| -> u16 {
                    text.lines().map(|l| {
                        // Estimate wrapping: use panel width
                        let width =  (chunks[1].width as usize).max(40);
                        let len = l.chars().count();
                        ((len + width - 1) / width).max(1) as u16
                    }).sum()
                };
                let area = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([
                        Constraint::Percentage(25), // File browser
                        Constraint::Percentage(40), // Metadata
                        Constraint::Percentage(35), // Image preview
                    ])
                    .split(area);

                // Left: File browser
                let file_items: Vec<ListItem> = files.iter().enumerate().map(|(i, f)| {
                    if i == selected {
                        ListItem::new(format!("> {} <", f)).style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
                    } else {
                        ListItem::new(f.to_string())
                    }
                }).collect();
                let left_border_style = if focused_panel == FocusedPanel::Left {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default()
                };
                let file_list = List::new(file_items)
                .block(Block::default()
                    .title(Span::styled(
                        "Files",
                        (if focused_panel == FocusedPanel::Left { Style::default().fg(Color::LightBlue) } else { Style::default().fg(Color::White) })
                            .add_modifier(Modifier::BOLD)
                    ))
                    .borders(Borders::ALL)
                    .border_style(left_border_style)
                    .title_alignment(Alignment::Center)
                )
                    .highlight_style(Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD));
                f.render_widget(file_list, chunks[0]);

                // Middle: Metadata (cached to avoid re-reading every frame)
                let mid_border_style = if focused_panel == FocusedPanel::Middle {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default()
                };
                let metadata_title_style = if focused_panel == FocusedPanel::Middle {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default().fg(Color::White)
                };
                // Always render a blank line at the end for clarity
                let mut metadata_with_blank = cached_metadata_text.clone();
                if !metadata_with_blank.ends_with('\n') {
                    metadata_with_blank.push('\n');
                }
                let metadata_for_render = metadata_with_blank.clone();
                let metadata_for_count = metadata_with_blank.clone();
                f.render_widget(
                    Paragraph::new(metadata_for_render)
                        .block(Block::default()
                            .title(Span::styled(
                                "Metadata",
                                metadata_title_style.add_modifier(Modifier::BOLD)
                            ))
                            .borders(Borders::ALL)
                            .border_style(mid_border_style)
                            .title_alignment(Alignment::Center)
                        )
                        .wrap(Wrap { trim: true })
                        .scroll((mid_scroll, 0)),
                    chunks[1],
                );

                // Calculate visible height for metadata panel (minus borders and title)
                visible_height = chunks[1].height.saturating_sub(2); // 1 for top border/title, 1 for bottom border
                total_lines = count_display_lines(&metadata_for_count);
                max_scroll = total_lines.saturating_sub(visible_height);

                // Right: Use image_panel module to render the right panel
                let file_name = files.get(selected).map(|s| s.as_str()).unwrap_or("");
                let right_panel_focused = focused_panel != FocusedPanel::Left && focused_panel != FocusedPanel::Middle;
                let image_panel_title_style = if !right_panel_focused {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::LightBlue)
                };
                let image_panel_block = Block::default()
                    .title(Span::styled(
                        "Image Preview",
                        image_panel_title_style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                    ))
                    .borders(Borders::ALL)
                    .title_alignment(Alignment::Center);
                f.render_widget(image_panel_block, chunks[2]);
                render_image_panel(f, chunks[2], file_name);
            })?;

            let poll_res = task::spawn_blocking(|| event::poll(std::time::Duration::from_millis(200))).await;
            if let Ok(Ok(true)) = poll_res {
                let read_res = task::spawn_blocking(|| event::read()).await;
                if let Ok(Ok(Event::Key(key))) = read_res {
                    match key.code {
                        KeyCode::Char('q') => running = false,
                        // Panel focus switching
                        KeyCode::Right | KeyCode::Char('l') => {
                            focused_panel = match focused_panel {
                                FocusedPanel::Left => FocusedPanel::Middle,
                                FocusedPanel::Middle => FocusedPanel::Left,
                            };
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            focused_panel = match focused_panel {
                                FocusedPanel::Left => FocusedPanel::Middle,
                                FocusedPanel::Middle => FocusedPanel::Left,
                            };
                        }
                        // Only allow up/down navigation when left panel is focused
                        KeyCode::Down if focused_panel == FocusedPanel::Left => {
                            if !files.is_empty() {
                                selected = (selected + 1) % files.len();
                            }
                        }
                        KeyCode::Up if focused_panel == FocusedPanel::Left => {
                            if !files.is_empty() {
                                if selected == 0 {
                                    selected = files.len() - 1;
                                } else {
                                    selected -= 1;
                                }
                            }
                        }
                    // Scroll metadata when middle panel is focused
                    KeyCode::Down if focused_panel == FocusedPanel::Middle => {
                        // Use max_scroll calculated above
                        if mid_scroll < max_scroll {
                            mid_scroll += 1;
                        }
                    }
                    KeyCode::Up if focused_panel == FocusedPanel::Middle => {
                        if mid_scroll > 0 {
                            mid_scroll -= 1;
                        }
                    }
                        _ => {}
                    }
                }
            }
        }

        let _ = terminal::disable_raw_mode();
        let _ = std::io::stdout().execute(terminal::LeaveAlternateScreen);
        Ok(())
    }
}
