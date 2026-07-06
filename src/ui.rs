use std::io;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::sync;
use crate::types::{Column, Issue, IssueState, Platform, Priority};

pub struct App {
    pub repo: String,
    pub platform: Platform,
    pub columns: Vec<Column>,
    pub selected_col: usize,
    pub status_msg: String,
    pub loading: bool,
    pub scroll_offset: u16,
}

impl App {
    pub fn new(repo: String, columns: Vec<Column>, platform: Platform) -> Self {
        App {
            repo,
            platform,
            columns,
            selected_col: 0,
            status_msg: String::new(),
            loading: true,
            scroll_offset: 0,
        }
    }

    pub fn selected_issue(&self) -> Option<&Issue> {
        let col = self.columns.get(self.selected_col)?;
        if col.issues.is_empty() {
            return None;
        }
        None
    }
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    repo: String,
    mut columns: Vec<Column>,
    platform: Platform,
) -> io::Result<()> {
    let mut app = App::new(repo, columns.clone(), platform);
    let mut col_issues: Vec<Vec<Issue>> = columns.iter().map(|_| vec![]).collect();
    let mut selected_row: usize = 0;

    match sync::fetch_issues(&app.repo, &app.platform) {
        Ok(issues) => {
            crate::config::write_cache(&issues, &crate::now_iso8601());
            for (i, col) in app.columns.iter_mut().enumerate() {
                col.issues = issues.iter().filter(|issue| col.matches(issue)).cloned().collect();
            }
            col_issues = app.columns.iter().map(|c| c.issues.clone()).collect();
            app.status_msg = format!("Loaded {} issues", issues.len());
        }
        Err(e) => {
            if let Some(cached) = crate::config::read_cache() {
                for (i, col) in app.columns.iter_mut().enumerate() {
                    col.issues = cached.iter().filter(|issue| col.matches(issue)).cloned().collect();
                }
                app.status_msg = format!("Offline: loaded {} cached issues", cached.len());
            } else {
                app.status_msg = format!("Error: {}", e);
            }
        }
    }
    app.loading = false;

    loop {
        for (i, col) in app.columns.iter_mut().enumerate() {
            if i < col_issues.len() {
                col.issues.clone_from(&col_issues[i]);
            }
        }

        terminal.draw(|f| draw(f, &mut app, selected_row))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => break,
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Some(col) = app.columns.get(app.selected_col) {
                        if !col.issues.is_empty() {
                            selected_row = (selected_row + 1).min(col.issues.len().saturating_sub(1));
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    selected_row = selected_row.saturating_sub(1);
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    app.selected_col = app.selected_col.saturating_sub(1);
                    selected_row = 0;
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    app.selected_col = (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
                    selected_row = 0;
                }
                KeyCode::Tab => {
                    app.selected_col = (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
                    selected_row = 0;
                }
                KeyCode::BackTab => {
                    app.selected_col = app.selected_col.saturating_sub(1);
                    selected_row = 0;
                }
                KeyCode::Char('r') => {
                    app.loading = true;
                    app.status_msg = "Refreshing...".into();
                    terminal.draw(|f| draw(f, &mut app, selected_row))?;

                    match sync::fetch_issues(&app.repo, &app.platform) {
                        Ok(issues) => {
                            crate::config::write_cache(&issues, &crate::now_iso8601());
                            for (i, col) in app.columns.iter_mut().enumerate() {
                                col.issues = issues.iter().filter(|issue| col.matches(issue)).cloned().collect();
                            }
                            col_issues = app.columns.iter().map(|c| c.issues.clone()).collect();
                            app.status_msg = format!("Refreshed: {} issues", issues.len());
                        }
                        Err(e) => {
                            app.status_msg = format!("Refresh failed: {}", e);
                        }
                    }
                    app.loading = false;
                }
                KeyCode::Enter => {
                    if let Some(col) = app.columns.get(app.selected_col) {
                        if selected_row < col.issues.len() {
                            let issue = &col.issues[selected_row];
                            sync::open_in_browser(&app.repo, issue.number, &app.platform);
                            app.status_msg = format!("#{} opened in browser", issue.number);
                        }
                    }
                }
                KeyCode::Char('n') => {
                    app.status_msg = "Enter title (type then Enter, or Esc to cancel):".into();
                    terminal.draw(|f| draw(f, &mut app, selected_row))?;
                    if let Some(title) = prompt_input("Title: ")? {
                        let _target_col = if app.columns.is_empty() {
                            "todo"
                        } else {
                            &app.columns[app.selected_col].id
                        };
                        let default_labels = if let Some(col) = app.columns.get(app.selected_col) {
                            if col.labels.is_empty() {
                                vec![]
                            } else {
                                vec![col.labels[0].clone()]
                            }
                        } else {
                            vec![]
                        };

                        match sync::create_issue(&app.repo, &title, &default_labels) {
                            Ok(num) => {
                                app.status_msg = format!("#{} created", num);
                                if let Ok(issues) = sync::fetch_issues(&app.repo, &app.platform) {
                                    for (i, col) in app.columns.iter_mut().enumerate() {
                                        col.issues = issues.iter().filter(|issue| col.matches(issue)).cloned().collect();
                                    }
                                    col_issues = app.columns.iter().map(|c| c.issues.clone()).collect();
                                }
                            }
                            Err(e) => app.status_msg = format!("Create failed: {}", e),
                        }
                    } else {
                        app.status_msg = "Cancelled".into();
                    }
                }
                KeyCode::Char('x') => {
                    if let Some(col) = app.columns.get(app.selected_col) {
                        if selected_row < col.issues.len() {
                            let issue = &col.issues[selected_row];
                            if issue.state == IssueState::Open {
                                match sync::close_issue(&app.repo, issue.number) {
                                    Ok(()) => {
                                        app.status_msg = format!("#{} closed", issue.number);
                                        if let Ok(issues) = sync::fetch_issues(&app.repo, &app.platform) {
                                            for (i, col) in app.columns.iter_mut().enumerate() {
                                                col.issues = issues.iter().filter(|issue| col.matches(issue)).cloned().collect();
                                            }
                                            col_issues = app.columns.iter().map(|c| c.issues.clone()).collect();
                                        }
                                    }
                                    Err(e) => app.status_msg = format!("Close failed: {}", e),
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('m') => {
                    if let Some(col) = app.columns.get(app.selected_col) {
                        if selected_row < col.issues.len() {
                            let issue = &col.issues[selected_row].clone();
                            let next_idx = (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
                            if next_idx != app.selected_col {
                                let next_col = &app.columns[next_idx];
                                let current_labels = &col.labels;
                                for lbl in current_labels {
                                    sync::remove_label(&app.repo, issue.number, lbl).ok();
                                }
                                for lbl in &next_col.labels {
                                    sync::add_label(&app.repo, issue.number, lbl).ok();
                                }
                                app.status_msg = format!("#{} moved to {}", issue.number, next_col.title);
                                if let Ok(issues) = sync::fetch_issues(&app.repo, &app.platform) {
                                    for (i, c) in app.columns.iter_mut().enumerate() {
                                        c.issues = issues.iter().filter(|issue| c.matches(issue)).cloned().collect();
                                    }
                                    col_issues = app.columns.iter().map(|c| c.issues.clone()).collect();
                                }
                                selected_row = 0;
                            }
                        }
                    }
                }
                KeyCode::Char('?') => {
                    app.status_msg = "h/l:nav  j/k:scroll  Enter:open  n:new  x:close  m:move  r:refresh  ?:help  q:quit".into();
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(f: &mut Frame, app: &mut App, selected_row: usize) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(area);

    let header = format!(
        " gh-kanban  {}  |  {}",
        app.repo,
        if app.loading { "Loading..." } else { &app.status_msg }
    );
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let header = Paragraph::new(header)
        .style(header_style)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(header, chunks[0]);

    if app.columns.is_empty() {
        return;
    }

    let n_cols = app.columns.len() as u16;
    let col_widths: Vec<Constraint> = (0..n_cols)
        .map(|_| Constraint::Ratio(1, n_cols.max(1).into()))
        .collect();

    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_widths)
        .split(chunks[1]);

    for (i, col) in app.columns.iter().enumerate() {
        if i >= col_chunks.len() {
            break;
        }
        let col_area = col_chunks[i];

        let is_selected = i == app.selected_col;
        let border_style = if is_selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let items: Vec<ListItem> = col
            .issues
            .iter()
            .enumerate()
            .map(|(idx, issue)| {
                let is_active = is_selected && idx == selected_row;

                let prio_indicator = match issue.priority {
                    Some(Priority::P0) => Span::styled(" ● ", Style::default().fg(Color::Red)),
                    Some(Priority::P1) => Span::styled(" ● ", Style::default().fg(Color::Yellow)),
                    Some(Priority::P2) => Span::styled(" ● ", Style::default().fg(Color::Green)),
                    Some(Priority::P3) => Span::styled(" ● ", Style::default().fg(Color::DarkGray)),
                    None => Span::raw("   "),
                };

                let num_str = format!("#{}", issue.number);
                let num = Span::styled(num_str, Style::default().fg(Color::Blue));

                let max_title = (col_area.width.saturating_sub(12)) as usize;
                let title = if issue.title.len() > max_title && max_title > 3 {
                    format!("{}...", &issue.title[..max_title.saturating_sub(3)])
                } else {
                    issue.title.clone()
                };
                let title_span = Span::styled(
                    title,
                    if is_active {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                );

                let assignee_str = if issue.assignees.is_empty() {
                    String::new()
                } else {
                    format!(" @{}", issue.assignees.join(", "))
                };
                let assignee = Span::styled(assignee_str, Style::default().fg(Color::DarkGray));

                let line = Line::from(vec![
                    prio_indicator,
                    num,
                    Span::raw(" "),
                    title_span,
                    assignee,
                ]);

                if is_active {
                    ListItem::new(line).style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ListItem::new(line)
                }
            })
            .collect();

        let col_title = format!("{} ({})", col.title, col.issues.len());

        let list = List::new(items).block(
            Block::default()
                .title(col_title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_alignment(ratatui::layout::Alignment::Center),
        );

        f.render_widget(list, col_area);
    }
}

fn prompt_input(prompt: &str) -> io::Result<Option<String>> {
    use std::io::Write;
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    println!();
                    return Ok(Some(input.trim().to_string()));
                }
                KeyCode::Esc => {
                    println!();
                    return Ok(None);
                }
                KeyCode::Char(c) => {
                    if !c.is_control() {
                        input.push(c);
                        print!("{}", c);
                        io::stdout().flush()?;
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                    print!("\x08 \x08");
                    io::stdout().flush()?;
                }
                _ => {}
            }
        }
    }
}
