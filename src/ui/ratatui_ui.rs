use std::path::PathBuf;
use anyhow::Result;
use crate::ui::app::{App, FocusedPanel};
use crate::ui::image_panel::render_image_panel;

pub struct RatatuiUI {
    app: App,
}

impl RatatuiUI {
    pub fn new() -> Self {
        RatatuiUI {
            app: App::new(),
        }
    }

    pub async fn run(&mut self, file: Option<PathBuf>) -> Result<()> {
        use ratatui::{prelude::*, widgets::*, layout::{Layout, Constraint, Direction}};
        use crossterm::{terminal, ExecutableCommand};
        use std::io::stdout;
        use tokio::task;
        use crossterm::event::{self, Event, KeyCode};
        use std::time::{Duration, Instant};
        use tokio::time::sleep;

        let footer_keys = vec![
            ("q", "quit", Color::White),
            ("d", "delete", Color::LightRed),
            ("c", "copy", Color::Green),
            ("space", "select", Color::Cyan),
            ("h/j/k/l", "nav", Color::White),
        ];

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

        // Branch: If its a file or not (Show Image TUI or Folder TUI) 
        // Single file
        let mut running = true;
        if let Some(ref path) = file {
            if path.is_file() {
                // Placeholder for a single file
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
        self.app.files = match std::fs::read_dir(dir) {
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

        while self.app.running {
            // Process any completed background image loads
            self.app.process_image_load_events();
            
            // Update metadata cache only when selection changes
            self.app.update_selection(dir);
            
            // Preload nearby images for smoother navigation
            self.app.preload_nearby_images(dir);

            // Calculate visible height for metadata panel (minus borders and title)
            let mut visible_height = 0u16;
            let mut max_scroll = 0u16;
            let mut total_lines = 0u16;
            
            // Update terminal dimensions for image loading
            let terminal_size = terminal.size()?;
            self.app.update_terminal_size(terminal_size.width, terminal_size.height);
            
            terminal.draw(|f| {
                let area = f.area();
                
                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([
                        Constraint::Min(3), 
                        Constraint::Length(2), // Footer
                    ])
                    .split(area);

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([
                        Constraint::Percentage(25), // File browser
                        Constraint::Percentage(40), // Metadata
                        Constraint::Percentage(35), // Image preview
                    ])
                    .split(main_chunks[0]);

                // Count display lines, including wrapped/multiline JSON
                let count_display_lines = |text: &str| -> u16 {
                    text.lines().map(|l| {
                        let width =  (chunks[1].width as usize).max(40);
                        let len = l.chars().count();
                        ((len + width - 1) / width).max(1) as u16
                    }).sum()
                };

                // Left: File browser
                let file_items: Vec<ListItem> = self.app.files.iter().enumerate().map(|(i, f)| {
                    if i == self.app.selected {
                        ListItem::new(format!("> {} <", f)).style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
                    } else {
                        ListItem::new(f.to_string())
                    }
                }).collect();
                let left_border_style = if self.app.focused_panel == FocusedPanel::Left {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default()
                };
                let file_list = List::new(file_items)
                .block(Block::default()
                    .title(Span::styled(
                        "Files",
                        (if self.app.focused_panel == FocusedPanel::Left { Style::default().fg(Color::LightBlue) } else { Style::default().fg(Color::White) })
                            .add_modifier(Modifier::BOLD)
                    ))
                    .borders(Borders::ALL)
                    .border_style(left_border_style)
                    .title_alignment(Alignment::Center)
                )
                    .highlight_style(Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD));
                f.render_widget(file_list, chunks[0]);

                // Middle: Metadata (cached)
                let mid_border_style = if self.app.focused_panel == FocusedPanel::Middle {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default()
                };
                let metadata_title_style = if self.app.focused_panel == FocusedPanel::Middle {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default().fg(Color::White)
                };
                // Always render a blank line at the end for clarity
                let mut metadata_with_blank = self.app.cached_metadata_text.clone();
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
                        .scroll((self.app.mid_scroll, 0)),
                    chunks[1],
                );

                // Calculate visible height for metadata panel (minus borders and title)
                visible_height = chunks[1].height.saturating_sub(2); // 1 for top border/title, 1 for bottom border
                total_lines = count_display_lines(&metadata_for_count);
                max_scroll = total_lines.saturating_sub(visible_height);

                // Right: Use image_panel module to render the right panel
                let file_name = self.app.files.get(self.app.selected).map(|s| s.as_str()).unwrap_or("");
                let image_panel_title_style = Style::default().fg(Color::White);
                let image_panel_block = Block::default()
                    .title(Span::styled(
                        "Image Preview",
                        image_panel_title_style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                    ))
                    .borders(Borders::ALL)
                    .title_alignment(Alignment::Center);
                f.render_widget(image_panel_block, chunks[2]);
                let load_status = self.app.get_image_load_status();
                let current_file_path = self.app.image_path.as_deref();
                render_image_panel(f, chunks[2], file_name, self.app.image_state.as_mut(), load_status, current_file_path);

                // Footer: keybindings
                let mut spans: Vec<Span> = Vec::new();
                for (i, (key, desc, color)) in footer_keys.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::raw("  "));
                    }
                    spans.push(Span::styled(
                        format!("{}", key),
                        Style::default().fg(*color).add_modifier(Modifier::BOLD)
                    ));
                    spans.push(Span::raw(":"));
                    spans.push(Span::styled(
                        format!("{}", desc),
                        Style::default().fg(Color::White)
                    ));
                }
                let footer = Paragraph::new(Line::from(spans))
                    .block(Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(Color::Gray))
                    )
                    .alignment(Alignment::Center);
                f.render_widget(footer, main_chunks[1]);
            })?;

            let now = Instant::now();
            let frame_time = Duration::from_millis(33); // aprox 30 fps
            if now.duration_since(self.app.last_frame_time) < frame_time {
                sleep(frame_time - now.duration_since(self.app.last_frame_time)).await;
            }
            self.app.last_frame_time = Instant::now();

            let poll_res = task::spawn_blocking(|| event::poll(std::time::Duration::from_millis(200))).await;
            if let Ok(Ok(true)) = poll_res {
                let read_res = task::spawn_blocking(|| event::read()).await;
                if let Ok(Ok(Event::Key(key))) = read_res {
                    // Handle scroll bounds for metadata panel
                    if key.code == crossterm::event::KeyCode::Down || key.code == crossterm::event::KeyCode::Char('j') {
                        if self.app.focused_panel == FocusedPanel::Middle {
                            if self.app.mid_scroll < max_scroll {
                                self.app.mid_scroll += 1;
                            }
                        }
                    }
                    self.app.handle_input(key.code, max_scroll, dir);
                }
            }
        }

        let _ = terminal::disable_raw_mode();
        let _ = std::io::stdout().execute(terminal::LeaveAlternateScreen);
        Ok(())
    }
}
