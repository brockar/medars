
use std::path::PathBuf;
use anyhow::Result;

pub struct RatatuiUI;

impl RatatuiUI {
    pub fn new() -> Self {
        RatatuiUI
    }

    pub async fn run(&mut self, file: Option<PathBuf>) -> Result<()> {
        use ratatui::{prelude::*, widgets::*};
        use crossterm::{terminal, ExecutableCommand};
        use std::io::stdout;
        use tokio::task;

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

        // Menu options
        let menu_items = vec![
            "✅ Detect metadata",
            "👁️ View metadata",
            "🗑️ Remove metadata",
            "🚪 Quit",
        ];
        let mut selected = 0;
        let mut output = String::new();

        use crossterm::event::{self, Event, KeyCode};
        let mut running = true;
        while running {
            terminal.draw(|f| {
                let area = f.area();
                // Horizontal split: left (menu/output), right (image)
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(2)
                    .constraints([
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ])
                    .split(area);

                // Left: vertical split for menu and output
                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(menu_items.len() as u16 + 2),
                        Constraint::Min(3),
                    ])
                    .split(chunks[0]);

                let items: Vec<ListItem> = menu_items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        if i == selected {
                            ListItem::new(format!("> {} <", item)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        } else {
                            ListItem::new(item.to_string())
                        }
                    })
                    .collect();
                let menu = List::new(items)
                    .block(Block::default().title("medars TUI").borders(Borders::ALL))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
                f.render_widget(menu, left_chunks[0]);

                let paragraph = Paragraph::new(output.clone())
                    .block(Block::default().title("Output").borders(Borders::ALL))
                    .wrap(Wrap { trim: true });
                f.render_widget(paragraph, left_chunks[1]);
            })?;

            // Use blocking for event polling and reading
            let poll_res = task::spawn_blocking(|| event::poll(std::time::Duration::from_millis(200))).await;
            if let Ok(Ok(true)) = poll_res {
                let read_res = task::spawn_blocking(|| event::read()).await;
                if let Ok(Ok(Event::Key(key))) = read_res {
                    match key.code {
                        KeyCode::Char('q') => running = false,
                        KeyCode::Down => {
                            selected = (selected + 1) % menu_items.len();
                        }
                        KeyCode::Up => {
                            if selected == 0 {
                                selected = menu_items.len() - 1;
                            } else {
                                selected -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            match selected {
                                0 => {
                                    if let Some(ref path) = file {
                                        let path = path.clone();
                                        let res = task::spawn_blocking(move || {
                                            crate::metadata::MetadataHandler::new().has_metadata(&path)
                                        }).await;
                                        output = match res {
                                            Ok(Ok(true)) => "✅ Image contains metadata".to_string(),
                                            Ok(Ok(false)) => "❌ No metadata found in image".to_string(),
                                            Ok(Err(e)) => format!("❌ Error: {}", e),
                                            Err(e) => format!("❌ Task error: {}", e),
                                        };
                                    } else {
                                        output = "No file provided.".to_string();
                                    }
                                }
                                1 => {
                                    if let Some(ref path) = file {
                                        let path = path.clone();
                                        let res = task::spawn_blocking(move || {
                                            crate::metadata::MetadataHandler::new().get_metadata_table(&path)
                                        }).await;
                                        output = match res {
                                            Ok(Ok(table)) => table,
                                            Ok(Err(e)) => format!("❌ Error: {}", e),
                                            Err(e) => format!("❌ Task error: {}", e),
                                        };
                                    } else {
                                        output = "No file provided.".to_string();
                                    }
                                }
                                2 => {
                                    if let Some(ref path) = file {
                                        let path = path.clone();
                                        let res = task::spawn_blocking(move || {
                                            crate::metadata::MetadataHandler::new().remove_metadata(&path, &path)
                                        }).await;
                                        output = match res {
                                            Ok(Ok(_)) => "✅ Metadata removed successfully".to_string(),
                                            Ok(Err(e)) => format!("❌ Error: {}", e),
                                            Err(e) => format!("❌ Task error: {}", e),
                                        };
                                    } else {
                                        output = "No file provided.".to_string();
                                    }
                                }
                                3 => running = false,
                                _ => output = String::new(),
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Always restore terminal state
        let _ = terminal::disable_raw_mode();
        let _ = std::io::stdout().execute(terminal::LeaveAlternateScreen);
        Ok(())
    }
}
