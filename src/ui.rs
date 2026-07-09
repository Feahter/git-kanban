use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;

use crate::sync;
use crate::types::{Backend, Column, Issue, IssueState, Priority};

pub struct App {
    pub repo: String,
    pub backend: Backend,
    pub columns: Vec<Column>,
    pub selected_col: usize,
    pub status_msg: String,
    pub loading: bool,
}

impl App {
    pub fn new(repo: String, backend: Backend, columns: Vec<Column>) -> Self {
        App {
            repo,
            backend,
            columns,
            selected_col: 0,
            status_msg: String::new(),
            loading: true,
        }
    }
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    repo: String,
    backend: Backend,
    columns: Vec<Column>,
) -> io::Result<()> {
    let mut app = App::new(repo, backend, columns.clone());
    let mut col_issues: Vec<Vec<Issue>> = columns.iter().map(|_| vec![]).collect();
    let mut selected_row: usize = 0;

    match sync::fetch_issues(app.backend, &app.repo) {
        Ok(issues) => {
            if !crate::config::write_cache(&issues, &crate::chrono_now(), &app.repo) {
                app.status_msg = format!("Warning: failed to write cache");
            }
            for (_, col) in app.columns.iter_mut().enumerate() {
                col.issues = issues
                    .iter()
                    .filter(|issue| col.matches(issue))
                    .cloned()
                    .collect();
            }
            col_issues = app.columns.iter().map(|c| c.issues.clone()).collect();
            app.status_msg = format!("Loaded {} issues", issues.len());
        }
        Err(e) => {
            if let Some(cached) = crate::config::read_cache(&app.repo) {
                for (_, col) in app.columns.iter_mut().enumerate() {
                    col.issues = cached
                        .iter()
                        .filter(|issue| col.matches(issue))
                        .cloned()
                        .collect();
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
                            selected_row =
                                (selected_row + 1).min(col.issues.len().saturating_sub(1));
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
                    app.selected_col =
                        (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
                    selected_row = 0;
                }
                KeyCode::Tab => {
                    app.selected_col =
                        (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
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

                    match sync::fetch_issues(app.backend, &app.repo) {
                        Ok(issues) => {
                            if !crate::config::write_cache(&issues, &crate::chrono_now(), &app.repo) {
                                app.status_msg = format!("Warning: failed to write cache");
                            }
                            for (_, col) in app.columns.iter_mut().enumerate() {
                                col.issues = issues
                                    .iter()
                                    .filter(|issue| col.matches(issue))
                                    .cloned()
                                    .collect();
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
                            sync::open_in_browser(app.backend, &app.repo, issue.number);
                            app.status_msg = format!("#{} opened in browser", issue.number);
                        }
                    }
                }
                KeyCode::Char('n') => {
                    app.status_msg =
                        "Enter title (type then Enter, or Esc to cancel):".into();
                    terminal.draw(|f| draw(f, &mut app, selected_row))?;
                    if let Some(title) = prompt_input("Title: ")? {
                        let default_labels = if let Some(col) = app.columns.get(app.selected_col) {
                            if col.labels.is_empty() {
                                vec![]
                            } else {
                                vec![col.labels[0].clone()]
                            }
                        } else {
                            vec![]
                        };

                        match sync::create_issue(app.backend, &app.repo, &title, None, &default_labels) {
                            Ok(num) => {
                                app.status_msg = format!("#{} created", num);
                                if let Ok(issues) = sync::fetch_issues(app.backend, &app.repo) {
                                    for (_, col) in app.columns.iter_mut().enumerate() {
                                        col.issues = issues
                                            .iter()
                                            .filter(|issue| col.matches(issue))
                                            .cloned()
                                            .collect();
                                    }
                                    col_issues =
                                        app.columns.iter().map(|c| c.issues.clone()).collect();
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
                                match sync::close_issue(app.backend, &app.repo, issue.number) {
                                    Ok(()) => {
                                        app.status_msg = format!("#{} closed", issue.number);
                                        if let Ok(issues) =
                                            sync::fetch_issues(app.backend, &app.repo)
                                        {
                                            for (_, col) in app.columns.iter_mut().enumerate() {
                                                col.issues = issues
                                                    .iter()
                                                    .filter(|issue| col.matches(issue))
                                                    .cloned()
                                                    .collect();
                                            }
                                            col_issues = app
                                                .columns
                                                .iter()
                                                .map(|c| c.issues.clone())
                                                .collect();
                                        }
                                    }
                                    Err(e) => app.status_msg = format!("Close failed: {}", e),
                                }
                            } else {
                                match sync::reopen_issue(app.backend, &app.repo, issue.number) {
                                    Ok(()) => {
                                        app.status_msg = format!("#{} reopened", issue.number);
                                        if let Ok(issues) =
                                            sync::fetch_issues(app.backend, &app.repo)
                                        {
                                            for (_, col) in app.columns.iter_mut().enumerate() {
                                                col.issues = issues
                                                    .iter()
                                                    .filter(|issue| col.matches(issue))
                                                    .cloned()
                                                    .collect();
                                            }
                                            col_issues = app
                                                .columns
                                                .iter()
                                                .map(|c| c.issues.clone())
                                                .collect();
                                        }
                                    }
                                    Err(e) => app.status_msg = format!("Reopen failed: {}", e),
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('m') => {
                    if let Some(col) = app.columns.get(app.selected_col) {
                        if selected_row < col.issues.len() {
                            let issue = &col.issues[selected_row].clone();
                            let next_idx =
                                (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
                            if next_idx != app.selected_col {
                                let next_col = &app.columns[next_idx];
                                // Only remove labels the issue actually has from this column
                                let removable: Vec<String> = col.labels.iter()
                                    .filter(|l| issue.labels.contains(l))
                                    .cloned()
                                    .collect();
                                match sync::move_issue(app.backend, &app.repo, issue.number, &removable, &next_col.labels) {
                                    Ok(()) => {
                                        app.status_msg =
                                            format!("#{} moved to {}", issue.number, next_col.title);
                                        if let Ok(issues) = sync::fetch_issues(app.backend, &app.repo) {
                                            for (_, c) in app.columns.iter_mut().enumerate() {
                                                c.issues = issues
                                                    .iter()
                                                    .filter(|issue| c.matches(issue))
                                                    .cloned()
                                                    .collect();
                                            }
                                            col_issues =
                                                app.columns.iter().map(|c| c.issues.clone()).collect();
                                        }
                                    }
                                    Err(e) => app.status_msg = format!("Move failed: {}", e),
                                }
                                selected_row = 0;
                            }
                        }
                    }
                }
                KeyCode::Char('M') => {
                    if let Some(col) = app.columns.get(app.selected_col) {
                        if selected_row < col.issues.len() {
                            let issue = &col.issues[selected_row].clone();
                            let prev_idx = app.selected_col.saturating_sub(1);
                            if prev_idx != app.selected_col {
                                let prev_col = &app.columns[prev_idx];
                                // Only remove labels the issue actually has from this column
                                let removable: Vec<String> = col.labels.iter()
                                    .filter(|l| issue.labels.contains(l))
                                    .cloned()
                                    .collect();
                                match sync::move_issue(app.backend, &app.repo, issue.number, &removable, &prev_col.labels) {
                                    Ok(()) => {
                                        app.status_msg =
                                            format!("#{} moved to {}", issue.number, prev_col.title);
                                        if let Ok(issues) = sync::fetch_issues(app.backend, &app.repo) {
                                            for (_, c) in app.columns.iter_mut().enumerate() {
                                                c.issues = issues
                                                    .iter()
                                                    .filter(|issue| c.matches(issue))
                                                    .cloned()
                                                    .collect();
                                            }
                                            col_issues =
                                                app.columns.iter().map(|c| c.issues.clone()).collect();
                                        }
                                    }
                                    Err(e) => app.status_msg = format!("Move failed: {}", e),
                                }
                                selected_row = 0;
                            }
                        }
                    }
                }
                KeyCode::Char('c') if key.modifiers != KeyModifiers::CONTROL => {
                    let issue_num = app.columns.get(app.selected_col)
                        .and_then(|col| col.issues.get(selected_row))
                        .map(|i| i.number);
                    if let Some(num) = issue_num {
                        app.status_msg = "Enter comment (type then Enter, or Esc to cancel):".into();
                        terminal.draw(|f| draw(f, &mut app, selected_row))?;
                        if let Some(body) = prompt_input("Comment: ")? {
                            let b = app.backend;
                            let r = app.repo.clone();
                            match sync::add_comment(b, &r, num, &body) {
                                Ok(()) => app.status_msg = format!("#{} commented", num),
                                Err(e) => app.status_msg = format!("Comment failed: {}", e),
                            }
                        } else {
                            app.status_msg = "Cancelled".into();
                        }
                    }
                }
                KeyCode::Char('a') => {
                    let issue_num = app.columns.get(app.selected_col)
                        .and_then(|col| col.issues.get(selected_row))
                        .map(|i| i.number);
                    if let Some(num) = issue_num {
                        let b = app.backend;
                        let r = app.repo.clone();
                        match sync::assign_self(b, &r, num) {
                            Ok(()) => {
                                app.status_msg = format!("#{} assigned to you", num);
                                if let Ok(issues) = sync::fetch_issues(b, &r) {
                                    for (_, col) in app.columns.iter_mut().enumerate() {
                                        col.issues = issues
                                            .iter()
                                            .filter(|issue| col.matches(issue))
                                            .cloned()
                                            .collect();
                                    }
                                    col_issues = app.columns.iter().map(|c| c.issues.clone()).collect();
                                }
                            }
                            Err(e) => app.status_msg = format!("Assign failed: {}", e),
                        }
                    }
                }
                KeyCode::Char('?') => {
                    app.status_msg =
                        "h/l:nav  j/k:scroll  Enter:open  n:new  x:close/reopen  m:move  M:move left  c:comment  a:assign me  r:refresh  ?:help  q:quit"
                            .into();
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
        " git-kanban  {}  |  {}",
        app.repo,
        if app.loading {
            "Loading..."
        } else {
            &app.status_msg
        }
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
                    Some(Priority::P3) => {
                        Span::styled(" ● ", Style::default().fg(Color::DarkGray))
                    }
                    None => Span::raw("   "),
                };

                let num_str = format!("#{}", issue.number);
                let num = Span::styled(num_str, Style::default().fg(Color::Blue));

                let max_title = (col_area.width.saturating_sub(12)) as usize;
                let title = if issue.title.len() > max_title && max_title > 3 {
                    let byte_end = issue.title.floor_char_boundary(max_title.saturating_sub(3));
                    format!("{}...", &issue.title[..byte_end])
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

                // Show first 3 non-column labels as tags [tag] after assignee
                let col_labels: &[String] = &col.labels;
                let extra_labels: Vec<&String> = issue
                    .labels
                    .iter()
                    .filter(|l| !col_labels.contains(l))
                    .take(3)
                    .collect();
                let label_tags = if extra_labels.is_empty() {
                    String::new()
                } else {
                    let available = col_area.width.saturating_sub(18) as usize;
                    let tags: String = extra_labels.iter().map(|l| format!(" [{}]", l)).collect();
                    if tags.len() > available && available > 8 {
                        let byte_end = tags.floor_char_boundary(available.saturating_sub(1));
                        format!(" {}", &tags[..byte_end])
                    } else if tags.len() > available {
                        String::new()
                    } else {
                        tags
                    }
                };
                let label_span = Span::styled(label_tags, Style::default().fg(Color::Magenta));

                let line = Line::from(vec![
                    prio_indicator,
                    num,
                    Span::raw(" "),
                    title_span,
                    assignee,
                    label_span,
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
                .title_alignment(Alignment::Center),
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
