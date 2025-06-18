use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        block::Title, Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Tabs,
    },
    Frame, Terminal,
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
struct Task {
    id: usize,
    title: String,
    completed: bool,
    created_at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug)]
enum PopupState {
    None,
    AddTask,
    Help,
}

struct App {
    tasks: Vec<Task>,
    selected_task: usize,
    list_state: ListState,
    input: String,
    input_mode: InputMode,
    popup_state: PopupState,
    tab_index: usize,
    progress: f64,
    last_tick: Instant,
}

impl Default for App {
    fn default() -> App {
        let mut app = App {
            tasks: vec![
                Task {
                    id: 1,
                    title: "Learn Rust TUI".to_string(),
                    completed: false,
                    created_at: chrono::Local::now(),
                },
                Task {
                    id: 2,
                    title: "Build awesome terminal app".to_string(),
                    completed: true,
                    created_at: chrono::Local::now(),
                },
                Task {
                    id: 3,
                    title: "Deploy to production".to_string(),
                    completed: false,
                    created_at: chrono::Local::now(),
                },
            ],
            selected_task: 0,
            list_state: ListState::default(),
            input: String::new(),
            input_mode: InputMode::Normal,
            popup_state: PopupState::None,
            tab_index: 0,
            progress: 0.0,
            last_tick: Instant::now(),
        };
        app.list_state.select(Some(0));
        app
    }
}

impl App {
    fn next_task(&mut self) {
        if !self.tasks.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= self.tasks.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
            self.selected_task = i;
        }
    }

    fn previous_task(&mut self) {
        if !self.tasks.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.tasks.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
            self.selected_task = i;
        }
    }

    fn toggle_task(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.tasks.len() {
                self.tasks[selected].completed = !self.tasks[selected].completed;
            }
        }
    }

    fn add_task(&mut self) {
        if !self.input.trim().is_empty() {
            let new_id = self.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            self.tasks.push(Task {
                id: new_id,
                title: self.input.clone(),
                completed: false,
                created_at: chrono::Local::now(),
            });
            self.input.clear();
            self.popup_state = PopupState::None;
            self.input_mode = InputMode::Normal;
        }
    }

    fn delete_selected_task(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.tasks.len() {
                self.tasks.remove(selected);
                if self.tasks.is_empty() {
                    self.list_state.select(None);
                } else if selected >= self.tasks.len() {
                    self.list_state.select(Some(self.tasks.len() - 1));
                    self.selected_task = self.tasks.len() - 1;
                }
            }
        }
    }

    fn completion_percentage(&self) -> f64 {
        if self.tasks.is_empty() {
            return 0.0;
        }
        let completed = self.tasks.iter().filter(|t| t.completed).count();
        (completed as f64 / self.tasks.len() as f64) * 100.0
    }

    fn update_progress(&mut self) {
        let target = self.completion_percentage();
        let diff = target - self.progress;
        self.progress += diff * 0.1; // Smooth animation
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('h') | KeyCode::F(1) => {
                                app.popup_state = PopupState::Help;
                            }
                            KeyCode::Char('a') => {
                                app.popup_state = PopupState::AddTask;
                                app.input_mode = InputMode::Editing;
                            }
                            KeyCode::Down | KeyCode::Char('j') => app.next_task(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous_task(),
                            KeyCode::Enter | KeyCode::Char(' ') => app.toggle_task(),
                            KeyCode::Char('d') => app.delete_selected_task(),
                            KeyCode::Tab => {
                                app.tab_index = (app.tab_index + 1) % 3;
                            }
                            KeyCode::Esc => {
                                app.popup_state = PopupState::None;
                            }
                            _ => {}
                        },
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => app.add_task(),
                            KeyCode::Char(c) => {
                                app.input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                            }
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                                app.popup_state = PopupState::None;
                                app.input.clear();
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update_progress();
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // Header with tabs
    let titles : Vec<Line> = ["Tasks", "Stats", "About"]
        .iter()
        .cloned()
        .map(Line::from)
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Rust TUI Demo"))
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        )
        .select(app.tab_index);
    f.render_widget(tabs, chunks[0]);

    // Main content based on selected tab
    match app.tab_index {
        0 => render_tasks_tab(f, app, chunks[1]),
        1 => render_stats_tab(f, app, chunks[1]),
        2 => render_about_tab(f, chunks[1]),
        _ => {}
    }

    // Footer
    let footer_text = match app.input_mode {
        InputMode::Normal => {
            "Press 'q' to quit, 'a' to add task, 'h' for help, ↑↓/jk to navigate, Space/Enter to toggle, 'd' to delete"
        }
        InputMode::Editing => "Press Esc to cancel, Enter to confirm",
    };
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);

    // Render popups
    match app.popup_state {
        PopupState::AddTask => render_add_task_popup(f, app, size),
        PopupState::Help => render_help_popup(f, size),
        PopupState::None => {}
    }
}

fn render_tasks_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Task list
    let tasks: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|task| {
            let status = if task.completed { "✓" } else { "○" };
            let style = if task.completed {
                Style::default().fg(Color::Green).add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", status), style),
                Span::styled(&task.title, style),
            ]))
        })
        .collect();

    let tasks_list = List::new(tasks)
        .block(Block::default().borders(Borders::ALL).title("Tasks"))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(tasks_list, chunks[0], &mut app.list_state);

    // Task details
    let selected_task = app.list_state.selected().and_then(|i| app.tasks.get(i));
    let details = if let Some(task) = selected_task {
        vec![
            Line::from(vec![Span::styled("ID: ", Style::default().fg(Color::Yellow)), Span::raw(task.id.to_string())]),
            Line::from(vec![Span::styled("Title: ", Style::default().fg(Color::Yellow)), Span::raw(&task.title)]),
            Line::from(vec![Span::styled("Status: ", Style::default().fg(Color::Yellow)), 
                Span::styled(
                    if task.completed { "Completed" } else { "Pending" },
                    if task.completed { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) }
                )]),
            Line::from(vec![Span::styled("Created: ", Style::default().fg(Color::Yellow)), 
                Span::raw(task.created_at.format("%Y-%m-%d %H:%M:%S").to_string())]),
        ]
    } else {
        vec![Line::from("No task selected")]
    };

    let details_paragraph = Paragraph::new(details)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(details_paragraph, chunks[1]);
}

fn render_stats_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Progress bar
    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Completion Progress"))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(app.progress as u16)
        .label(format!("{:.1}%", app.progress));
    f.render_widget(progress, chunks[0]);

    // Statistics
    let total_tasks = app.tasks.len();
    let completed_tasks = app.tasks.iter().filter(|t| t.completed).count();
    let pending_tasks = total_tasks - completed_tasks;

    let stats_text = vec![
        Line::from(vec![
            Span::styled("Total Tasks: ", Style::default().fg(Color::Cyan)),
            Span::styled(total_tasks.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Completed: ", Style::default().fg(Color::Green)),
            Span::styled(completed_tasks.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Pending: ", Style::default().fg(Color::Red)),
            Span::styled(pending_tasks.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from("Recent Activity:"),
    ];

    let mut recent_tasks: Vec<_> = app.tasks.iter().collect();
    recent_tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    
    let mut all_text = stats_text;
    for task in recent_tasks.iter().take(5) {
        let status = if task.completed { "✓" } else { "○" };
        all_text.push(Line::from(vec![
            Span::styled(format!("{} ", status), 
                if task.completed { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Yellow) }),
            Span::raw(&task.title),
        ]));
    }

    let stats = Paragraph::new(all_text)
        .block(Block::default().borders(Borders::ALL).title("Statistics"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(stats, chunks[1]);
}

fn render_about_tab(f: &mut Frame, area: Rect) {
    let about_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Rust TUI Demo Application", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from("Built with:"),
        Line::from(vec![
            Span::raw("• "),
            Span::styled("ratatui", Style::default().fg(Color::Cyan)),
            Span::raw(" - Terminal UI library"),
        ]),
        Line::from(vec![
            Span::raw("• "),
            Span::styled("crossterm", Style::default().fg(Color::Cyan)),
            Span::raw(" - Cross-platform terminal manipulation"),
        ]),
        Line::from(vec![
            Span::raw("• "),
            Span::styled("chrono", Style::default().fg(Color::Cyan)),
            Span::raw(" - Date and time handling"),
        ]),
        Line::from(""),
        Line::from("Features:"),
        Line::from("• Task management with completion tracking"),
        Line::from("• Interactive navigation with keyboard shortcuts"),
        Line::from("• Real-time progress visualization"),
        Line::from("• Multi-tab interface"),
        Line::from("• Popup dialogs and help system"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" to switch between tabs"),
        ]),
    ];

    let about = Paragraph::new(about_text)
        .block(Block::default().borders(Borders::ALL).title("About"))
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(about, area);
}

fn render_add_task_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    let input_area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(popup_area);

    f.render_widget(Clear, popup_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Add New Task")
            .style(Style::default().bg(Color::Black)),
        popup_area,
    );
    let text = app.input.clone();
    let input = Paragraph::new(text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Task Title"));
    f.render_widget(input, input_area[0]);

    let help_text = Paragraph::new("Press Enter to add task, Esc to cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help_text, input_area[1]);

    // Show cursor
    f.set_cursor(
        input_area[0].x + app.input.len() as u16 + 1,
        input_area[0].y + 1,
    );
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 70, area);
    f.render_widget(Clear, popup_area);
    
    let help_text = vec![
        Line::from(vec![
            Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("           Quit application"),
        ]),
        Line::from(vec![
            Span::styled("h, F1", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("       Show this help"),
        ]),
        Line::from(vec![
            Span::styled("a", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("           Add new task"),
        ]),
        Line::from(vec![
            Span::styled("d", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("           Delete selected task"),
        ]),
        Line::from(vec![
            Span::styled("↑/↓, j/k", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("    Navigate tasks"),
        ]),
        Line::from(vec![
            Span::styled("Space/Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("  Toggle task completion"),
        ]),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("         Switch tabs"),
        ]),
        Line::from(vec![
            Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("         Close popups"),
        ]),
        Line::from(""),
        Line::from("Press Esc to close this help"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(help, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}