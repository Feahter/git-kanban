use std::collections::HashMap;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;

use crate::sync;
use crate::types::{Backend, Column, IssueState, Priority};

pub struct App {
    pub repos: Vec<String>,
    pub active_repo: String,
    pub backend: Backend,
    pub columns: Vec<Column>,
    pub repo_cache: Vec<(String, Vec<Column>)>,
    pub status_msg: String,
    pub selected_col: usize,
    pub selected_row: usize,
    pub selected_repo_idx: usize,
    pub show_sidebar: bool,
    pub sidebar_focused: bool,
    pub loading: bool,
    /// Per-repo selected_row (keyed by repo name)
    pub repo_selected_rows: HashMap<String, usize>,
}

impl App {
    pub fn new(repos: Vec<String>, backend: Backend, columns_template: Vec<Column>) -> Self {
        let show_sidebar = repos.len() > 1;
        let active_repo = repos.first().cloned().unwrap_or_default();
        // Pre-fill repo_cache with empty columns for each repo
        let repo_cache: Vec<(String, Vec<Column>)> = repos
            .iter()
            .map(|r| (r.clone(), Self::empty_columns(&columns_template)))
            .collect();
        App {
            repos,
            active_repo,
            backend,
            columns: columns_template,
            repo_cache,
            status_msg: String::new(),
            selected_col: 0,
            selected_row: 0,
            selected_repo_idx: 0,
            show_sidebar,
            sidebar_focused: false,
            loading: true,
            repo_selected_rows: HashMap::new(),
        }
    }

    fn empty_columns(template: &[Column]) -> Vec<Column> {
        template
            .iter()
            .map(|c| Column {
                id: c.id.clone(),
                title: c.title.clone(),
                labels: c.labels.clone(),
                show_closed: c.show_closed,
                issues: vec![],
            })
            .collect()
    }

    /// Load issues for a repo into the given columns, trying cache first.
    fn load_repo_columns(
        backend: Backend,
        repo: &str,
        columns_template: &[Column],
        status_msg: &mut String,
    ) -> Vec<Column> {
        let mut cols = Self::empty_columns(columns_template);
        match sync::fetch_issues(backend, repo) {
            Ok(issues) => {
                if !crate::config::write_cache(&issues, &crate::chrono_now(), repo) {
                    status_msg.push_str(&format!("Warning: failed to write cache for {}", repo));
                }
                for col in cols.iter_mut() {
                    col.issues = issues
                        .iter()
                        .filter(|issue| col.matches(issue))
                        .cloned()
                        .collect();
                }
                if status_msg.is_empty() {
                    status_msg.push_str(&format!("Loaded {} issues from {}", issues.len(), repo));
                }
            }
            Err(e) => {
                if let Some(cached) = crate::config::read_cache(repo) {
                    for col in cols.iter_mut() {
                        col.issues = cached
                            .iter()
                            .filter(|issue| col.matches(issue))
                            .cloned()
                            .collect();
                    }
                    status_msg.push_str(&format!(
                        "Offline: loaded {} cached issues from {}",
                        cached.len(),
                        repo
                    ));
                } else {
                    status_msg.push_str(&format!("Error loading {}: {}", repo, e));
                }
            }
        }
        cols
    }
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    repos: Vec<String>,
    backend: Backend,
    columns: Vec<Column>,
) -> io::Result<()> {
    let mut app = App::new(repos, backend, columns);

    // Build a template for empty columns we can clone
    let columns_template = app.columns.clone();

    // Load data for all repos
    for i in 0..app.repos.len() {
        let repo = app.repos[i].clone();
        let cols = App::load_repo_columns(app.backend, &repo, &columns_template, &mut app.status_msg);
        app.repo_cache[i] = (repo.clone(), cols);
    }

    // Set active repo to first one
    if !app.repo_cache.is_empty() {
        app.columns = app.repo_cache[0].1.clone();
        app.active_repo = app.repos[0].clone();
        app.selected_repo_idx = 0;
    }
    app.loading = false;

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => break,

                // ── Sidebar navigation (only when multi-repo) ──
                KeyCode::Tab if app.show_sidebar => {
                    app.sidebar_focused = !app.sidebar_focused;
                    if !app.sidebar_focused {
                        // When leaving sidebar, ensure active_repo matches selection
                        app.switch_to_repo(app.selected_repo_idx, &columns_template);
                        app.status_msg = format!(
                            "Switched to {}",
                            app.active_repo
                        );
                    }
                }

                // ── Sidebar focused keys ──
                _ if app.sidebar_focused => {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if app.selected_repo_idx + 1 < app.repos.len() {
                                app.selected_repo_idx += 1;
                                app.switch_to_repo(app.selected_repo_idx, &columns_template);
                                app.status_msg = format!("Switched to {}", app.active_repo);
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.selected_repo_idx > 0 {
                                app.selected_repo_idx -= 1;
                                app.switch_to_repo(app.selected_repo_idx, &columns_template);
                                app.status_msg = format!("Switched to {}", app.active_repo);
                            }
                        }
                        KeyCode::Enter => {
                            app.sidebar_focused = false;
                            app.switch_to_repo(app.selected_repo_idx, &columns_template);
                            app.status_msg = format!("Switched to {}", app.active_repo);
                        }
                        _ => {}
                    }
                }

                // ── Kanban focused keys (or single-repo mode) ──
                _ => {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if let Some(col) = app.columns.get(app.selected_col) {
                                if !col.issues.is_empty() {
                                    app.selected_row =
                                        (app.selected_row + 1).min(col.issues.len().saturating_sub(1));
                                }
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.selected_row = app.selected_row.saturating_sub(1);
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            app.selected_col = app.selected_col.saturating_sub(1);
                            app.selected_row = 0;
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            app.selected_col =
                                (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
                            app.selected_row = 0;
                        }
                        // Tab in single-repo mode switches columns
                        KeyCode::Tab => {
                            app.selected_col =
                                (app.selected_col + 1).min(app.columns.len().saturating_sub(1));
                            app.selected_row = 0;
                        }
                        KeyCode::BackTab => {
                            app.selected_col = app.selected_col.saturating_sub(1);
                            app.selected_row = 0;
                        }
                        KeyCode::Char('r') => {
                            app.loading = true;
                            app.status_msg = format!("Refreshing {}...", app.active_repo);
                            terminal.draw(|f| draw(f, &mut app))?;

                            let repo = app.active_repo.clone();
                            let cols = App::load_repo_columns(app.backend, &repo, &columns_template, &mut app.status_msg);
                            // Update both cache and active columns
                            for (ref r, ref mut cache_cols) in app.repo_cache.iter_mut() {
                                if r == &repo {
                                    *cache_cols = cols.clone();
                                    break;
                                }
                            }
                            if app.active_repo == repo {
                                app.columns = cols;
                            }
                            app.loading = false;
                        }
                        KeyCode::Char('R') => {
                            // Refresh all repos
                            app.loading = true;
                            app.status_msg = "Refreshing all repos...".into();
                            terminal.draw(|f| draw(f, &mut app))?;

                            for i in 0..app.repos.len() {
                                let repo = app.repos[i].clone();
                                let cols = App::load_repo_columns(
                                    app.backend,
                                    &repo,
                                    &columns_template,
                                    &mut app.status_msg,
                                );
                                app.repo_cache[i] = (repo.clone(), cols.clone());
                                if repo == app.active_repo {
                                    app.columns = cols;
                                }
                            }
                            app.loading = false;
                        }
                        KeyCode::Enter => {
                            if let Some(col) = app.columns.get(app.selected_col) {
                                if app.selected_row < col.issues.len() {
                                    let issue = &col.issues[app.selected_row];
                                    sync::open_in_browser(
                                        app.backend,
                                        &app.active_repo,
                                        issue.number,
                                    );
                                    app.status_msg =
                                        format!("#{} opened in browser", issue.number);
                                }
                            }
                        }
                        KeyCode::Char('n') => {
                            app.status_msg =
                                "Enter title (type then Enter, or Esc to cancel):".into();
                            terminal.draw(|f| draw(f, &mut app))?;
                            if let Some(title) = prompt_input("Title: ")? {
                                let default_labels =
                                    if let Some(col) = app.columns.get(app.selected_col) {
                                        if col.labels.is_empty() {
                                            vec![]
                                        } else {
                                            vec![col.labels[0].clone()]
                                        }
                                    } else {
                                        vec![]
                                    };

                                match sync::create_issue(
                                    app.backend,
                                    &app.active_repo,
                                    &title,
                                    None,
                                    &default_labels,
                                ) {
                                    Ok(num) => {
                                        app.status_msg = format!("#{} created", num);
                                        if let Ok(issues) =
                                            sync::fetch_issues(app.backend, &app.active_repo)
                                        {
                                            for col in app.columns.iter_mut() {
                                                col.issues = issues
                                                    .iter()
                                                    .filter(|issue| col.matches(issue))
                                                    .cloned()
                                                    .collect();
                                            }
                                            // Update cache
                                            for (ref r, ref mut cache_cols) in
                                                app.repo_cache.iter_mut()
                                            {
                                                if r == &app.active_repo {
                                                    *cache_cols = app.columns.clone();
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        app.status_msg = format!("Create failed: {}", e)
                                    }
                                }
                            } else {
                                app.status_msg = "Cancelled".into();
                            }
                        }
                        KeyCode::Char('x') => {
                            if let Some(col) = app.columns.get(app.selected_col) {
                                if app.selected_row < col.issues.len() {
                                    let issue = &col.issues[app.selected_row];
                                    if issue.state == IssueState::Open {
                                        match sync::close_issue(
                                            app.backend,
                                            &app.active_repo,
                                            issue.number,
                                        ) {
                                            Ok(()) => {
                                                app.status_msg =
                                                    format!("#{} closed", issue.number);
                                                if let Ok(issues) =
                                                    sync::fetch_issues(app.backend, &app.active_repo)
                                                {
                                                    for col in app.columns.iter_mut() {
                                                        col.issues = issues
                                                            .iter()
                                                            .filter(|i| col.matches(i))
                                                            .cloned()
                                                            .collect();
                                                    }
                                                    // Update cache
                                                    for (ref r, ref mut cache_cols) in
                                                        app.repo_cache.iter_mut()
                                                    {
                                                        if r == &app.active_repo {
                                                            *cache_cols = app.columns.clone();
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.status_msg = format!("Close failed: {}", e)
                                            }
                                        }
                                    } else {
                                        match sync::reopen_issue(
                                            app.backend,
                                            &app.active_repo,
                                            issue.number,
                                        ) {
                                            Ok(()) => {
                                                app.status_msg =
                                                    format!("#{} reopened", issue.number);
                                                if let Ok(issues) =
                                                    sync::fetch_issues(app.backend, &app.active_repo)
                                                {
                                                    for col in app.columns.iter_mut() {
                                                        col.issues = issues
                                                            .iter()
                                                            .filter(|i| col.matches(i))
                                                            .cloned()
                                                            .collect();
                                                    }
                                                    // Update cache
                                                    for (ref r, ref mut cache_cols) in
                                                        app.repo_cache.iter_mut()
                                                    {
                                                        if r == &app.active_repo {
                                                            *cache_cols = app.columns.clone();
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.status_msg =
                                                    format!("Reopen failed: {}", e)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('m') => {
                            if let Some(col) = app.columns.get(app.selected_col) {
                                if app.selected_row < col.issues.len() {
                                    let issue = &col.issues[app.selected_row].clone();
                                    let next_idx = (app.selected_col + 1)
                                        .min(app.columns.len().saturating_sub(1));
                                    if next_idx != app.selected_col {
                                        let next_col = &app.columns[next_idx];
                                        // Only remove labels the issue actually has from this column
                                        let removable: Vec<String> = col
                                            .labels
                                            .iter()
                                            .filter(|l| issue.labels.contains(l))
                                            .cloned()
                                            .collect();
                                        match sync::move_issue(
                                            app.backend,
                                            &app.active_repo,
                                            issue.number,
                                            &removable,
                                            &next_col.labels,
                                        ) {
                                            Ok(()) => {
                                                app.status_msg = format!(
                                                    "#{} moved to {}",
                                                    issue.number, next_col.title
                                                );
                                                if let Ok(issues) =
                                                    sync::fetch_issues(app.backend, &app.active_repo)
                                                {
                                                    for c in app.columns.iter_mut() {
                                                        c.issues = issues
                                                            .iter()
                                                            .filter(|i| c.matches(i))
                                                            .cloned()
                                                            .collect();
                                                    }
                                                    // Update cache
                                                    for (ref r, ref mut cache_cols) in
                                                        app.repo_cache.iter_mut()
                                                    {
                                                        if r == &app.active_repo {
                                                            *cache_cols = app.columns.clone();
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.status_msg =
                                                    format!("Move failed: {}", e)
                                            }
                                        }
                                        app.selected_row = 0;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('M') => {
                            if let Some(col) = app.columns.get(app.selected_col) {
                                if app.selected_row < col.issues.len() {
                                    let issue = &col.issues[app.selected_row].clone();
                                    let prev_idx = app.selected_col.saturating_sub(1);
                                    if prev_idx != app.selected_col {
                                        let prev_col = &app.columns[prev_idx];
                                        // Only remove labels the issue actually has from this column
                                        let removable: Vec<String> = col
                                            .labels
                                            .iter()
                                            .filter(|l| issue.labels.contains(l))
                                            .cloned()
                                            .collect();
                                        match sync::move_issue(
                                            app.backend,
                                            &app.active_repo,
                                            issue.number,
                                            &removable,
                                            &prev_col.labels,
                                        ) {
                                            Ok(()) => {
                                                app.status_msg = format!(
                                                    "#{} moved to {}",
                                                    issue.number, prev_col.title
                                                );
                                                if let Ok(issues) =
                                                    sync::fetch_issues(app.backend, &app.active_repo)
                                                {
                                                    for c in app.columns.iter_mut() {
                                                        c.issues = issues
                                                            .iter()
                                                            .filter(|i| c.matches(i))
                                                            .cloned()
                                                            .collect();
                                                    }
                                                    // Update cache
                                                    for (ref r, ref mut cache_cols) in
                                                        app.repo_cache.iter_mut()
                                                    {
                                                        if r == &app.active_repo {
                                                            *cache_cols = app.columns.clone();
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.status_msg =
                                                    format!("Move failed: {}", e)
                                            }
                                        }
                                        app.selected_row = 0;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('c') if key.modifiers != KeyModifiers::CONTROL => {
                            let issue_num = app
                                .columns
                                .get(app.selected_col)
                                .and_then(|col| col.issues.get(app.selected_row))
                                .map(|i| i.number);
                            if let Some(num) = issue_num {
                                app.status_msg =
                                    "Enter comment (type then Enter, or Esc to cancel):".into();
                                terminal.draw(|f| draw(f, &mut app))?;
                                if let Some(body) = prompt_input("Comment: ")? {
                                    match sync::add_comment(
                                        app.backend,
                                        &app.active_repo,
                                        num,
                                        &body,
                                    ) {
                                        Ok(()) => {
                                            app.status_msg = format!("#{} commented", num)
                                        }
                                        Err(e) => {
                                            app.status_msg =
                                                format!("Comment failed: {}", e)
                                        }
                                    }
                                } else {
                                    app.status_msg = "Cancelled".into();
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            let issue_num = app
                                .columns
                                .get(app.selected_col)
                                .and_then(|col| col.issues.get(app.selected_row))
                                .map(|i| i.number);
                            if let Some(num) = issue_num {
                                match sync::assign_self(app.backend, &app.active_repo, num) {
                                    Ok(()) => {
                                        app.status_msg =
                                            format!("#{} assigned to you", num);
                                        if let Ok(issues) =
                                            sync::fetch_issues(app.backend, &app.active_repo)
                                        {
                                            for col in app.columns.iter_mut() {
                                                col.issues = issues
                                                    .iter()
                                                    .filter(|i| col.matches(i))
                                                    .cloned()
                                                    .collect();
                                            }
                                            // Update cache
                                            for (ref r, ref mut cache_cols) in
                                                app.repo_cache.iter_mut()
                                            {
                                                if r == &app.active_repo {
                                                    *cache_cols = app.columns.clone();
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        app.status_msg = format!("Assign failed: {}", e)
                                    }
                                }
                            }
                        }
                        KeyCode::Char('?') => {
                            app.status_msg = if app.show_sidebar {
                                "h/l:nav  j/k:scroll  Tab:sidebar  Enter:open  n:new  x:close/reopen  m:move  M:move left  c:comment  a:assign me  r:refresh  R:refresh all  ?:help  q:quit"
                            } else {
                                "h/l:nav  j/k:scroll  Enter:open  n:new  x:close/reopen  m:move  M:move left  c:comment  a:assign me  r:refresh  ?:help  q:quit"
                            }
                            .into();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

impl App {
    /// Switch to a repo by index, saving/restoring selected_row per repo.
    fn switch_to_repo(&mut self, repo_idx: usize, columns_template: &[Column]) {
        if repo_idx >= self.repos.len() {
            return;
        }
        let new_repo = self.repos[repo_idx].clone();

        // Save current selected_row for old repo
        if !self.active_repo.is_empty() {
            self.repo_selected_rows
                .insert(self.active_repo.clone(), self.selected_row);
        }

        // Find or load from repo_cache
        if let Some((_, ref cols)) = self
            .repo_cache
            .iter()
            .find(|(r, _)| r == &new_repo)
        {
            self.columns = cols.clone();
        } else {
            // Load fresh
            let cols = App::load_repo_columns(
                self.backend,
                &new_repo,
                columns_template,
                &mut self.status_msg,
            );
            self.repo_cache.push((new_repo.clone(), cols.clone()));
            self.columns = cols;
        }

        self.active_repo = new_repo;
        self.selected_repo_idx = repo_idx;
        // Restore selected_row for new repo
        self.selected_row = self
            .repo_selected_rows
            .get(&self.active_repo)
            .copied()
            .unwrap_or(0);
    }
}

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(area);

    // ── Header ──
    let header = if app.show_sidebar {
        format!(
            " git-kanban  {}  |  {}  (Tab: toggle focus)",
            app.active_repo,
            if app.loading {
                "Loading..."
            } else {
                &app.status_msg
            }
        )
    } else {
        format!(
            " git-kanban  {}  |  {}",
            app.active_repo,
            if app.loading {
                "Loading..."
            } else {
                &app.status_msg
            }
        )
    };
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

    // ── Sidebar + kanban layout (multi-repo) or just kanban (single-repo) ──
    if app.show_sidebar {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(22), Constraint::Min(0)])
            .split(chunks[1]);
        draw_sidebar(f, app, layout[0]);
        draw_kanban(f, app, layout[1]);
    } else {
        draw_kanban(f, app, chunks[1]);
    }
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let is_selected = i == app.selected_repo_idx;
            let is_active = repo == &app.active_repo;
            let prefix = if is_active {
                " ● "
            } else if is_selected {
                " ◉ "
            } else {
                "   "
            };
            let line = Line::from(Span::raw(format!("{}{}", prefix, repo)));
            let style = if is_selected && app.sidebar_focused {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if is_active && is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let border_style = if app.sidebar_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items).block(
        Block::default()
            .title(" Repos ")
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    f.render_widget(list, area);
}

fn draw_kanban(f: &mut Frame, app: &App, area: Rect) {
    let n_cols = app.columns.len() as u16;
    let col_widths: Vec<Constraint> = (0..n_cols)
        .map(|_| Constraint::Ratio(1, n_cols.max(1).into()))
        .collect();

    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_widths)
        .split(area);

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
                let is_active = is_selected && idx == app.selected_row;

                let prio_indicator = match issue.priority {
                    Some(Priority::P0) => {
                        Span::styled(" ● ", Style::default().fg(Color::Red))
                    }
                    Some(Priority::P1) => {
                        Span::styled(" ● ", Style::default().fg(Color::Yellow))
                    }
                    Some(Priority::P2) => {
                        Span::styled(" ● ", Style::default().fg(Color::Green))
                    }
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
                let assignee =
                    Span::styled(assignee_str, Style::default().fg(Color::DarkGray));

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
                    let tags: String =
                        extra_labels.iter().map(|l| format!(" [{}]", l)).collect();
                    if tags.len() > available && available > 8 {
                        let byte_end = tags.floor_char_boundary(available.saturating_sub(1));
                        format!(" {}", &tags[..byte_end])
                    } else if tags.len() > available {
                        String::new()
                    } else {
                        tags
                    }
                };
                let label_span =
                    Span::styled(label_tags, Style::default().fg(Color::Magenta));

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
