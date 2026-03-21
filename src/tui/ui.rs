/// Ratatui render function — three-pane layout: input / results / status.
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::App;

/// Render the full TUI frame.
pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // query input
            Constraint::Min(0),    // results
            Constraint::Length(1), // status bar
        ])
        .split(f.size());

    render_input(f, app, chunks[0]);
    render_results(f, app, chunks[1]);
    render_status(f, app, chunks[2]);
}

/// Render the query-input block (top pane).
fn render_input(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Query ")
        .border_style(Style::default().fg(Color::Cyan));

    // Split query at cursor to display cursor marker
    let before = &app.query[..app.cursor_pos];
    let after = &app.query[app.cursor_pos..];
    let line = Line::from(vec![
        Span::raw(before),
        Span::styled("█", Style::default().fg(Color::Cyan)),
        Span::raw(after),
    ]);

    let para = Paragraph::new(line).block(block);
    f.render_widget(para, area);

    // Position the real terminal cursor for accessibility / copy-paste tools
    let cursor_x = area.x + 1 + app.cursor_pos as u16;
    let cursor_y = area.y + 1;
    if cursor_x < area.x + area.width.saturating_sub(1) {
        f.set_cursor(cursor_x, cursor_y);
    }
}

/// Render the results pane (middle, scrollable).
fn render_results(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let (title, title_style) = if let Some(ref err) = app.error {
        let first_line = err.lines().next().unwrap_or("eval error");
        (
            format!(" Error: {first_line} "),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else {
        (
            format!(" Results ({}) ", app.results.len()),
            Style::default().fg(Color::Green),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(title_style);

    let lines: Vec<Line> = app
        .results
        .iter()
        .map(|r| {
            let s = serde_json::to_string(&r.fields).unwrap_or_default();
            Line::from(s)
        })
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .scroll((app.scroll as u16, 0));
    f.render_widget(para, area);
}

/// Render the one-line status bar (bottom).
fn render_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let style = Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let text = format!(
        "  {}    ↑↓/PgUp/PgDn scroll · ←→ cursor · Esc/Ctrl+C quit",
        app.status
    );
    let para = Paragraph::new(text).style(style);
    f.render_widget(para, area);
}
