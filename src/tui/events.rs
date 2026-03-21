/// Crossterm event loop — handles keyboard input and drives the TUI.
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::app::App;
use super::ui;
use crate::util::error::{QkError, Result};

/// Initialise the terminal, run the event loop, then restore the terminal.
///
/// The terminal is always restored even if the loop returns an error.
pub fn run(mut app: App) -> Result<()> {
    enable_raw_mode().map_err(tui_io_err)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(tui_io_err)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(tui_io_err)?;

    let result = event_loop(&mut terminal, &mut app);

    // Always restore the terminal, even on error
    disable_raw_mode().map_err(tui_io_err)?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(tui_io_err)?;
    terminal.show_cursor().map_err(tui_io_err)?;

    result
}

/// Poll for events with a 100 ms timeout and redraw each iteration.
fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app)).map_err(tui_io_err)?;

        if !event::poll(Duration::from_millis(100)).map_err(tui_io_err)? {
            continue;
        }

        if let Event::Key(key) = event::read().map_err(tui_io_err)? {
            handle_key(app, key.code, key.modifiers);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

/// Dispatch a single key event to the App; re-evaluates if the query changed.
fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let prev_query = app.query.clone();

    match code {
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Char(c) => app.insert_char(c),
        KeyCode::Backspace => app.delete_char_before(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Up => app.scroll_up(1),
        KeyCode::Down => app.scroll_down(1),
        KeyCode::PageUp => app.scroll_up(20),
        KeyCode::PageDown => app.scroll_down(20),
        _ => {}
    }

    if app.query != prev_query {
        app.eval();
    }
}

/// Wrap any `io::Error`-like error into `QkError::Io`.
fn tui_io_err(e: impl std::fmt::Display) -> QkError {
    QkError::Io {
        path: "<tui>".to_string(),
        source: io::Error::other(e.to_string()),
    }
}
