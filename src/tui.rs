//! Terminal integration: key mapping and the event loop.

use std::io;
use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, Input, Overlay};
use crate::ui::{self, View};

/// How often the screen is refreshed while idle (drives the exam clock).
const TICK: Duration = Duration::from_millis(250);

/// What a key press means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Forward to the application.
    Input(Input),
    /// Scroll the matrix panel by this many columns/lines.
    Scroll {
        /// Horizontal offset change.
        dx: i32,
        /// Vertical offset change.
        dy: i32,
    },
    /// Do nothing.
    Ignore,
}

/// Translates a key event into an [`Action`].
#[must_use]
pub fn map_key(key: KeyEvent) -> Action {
    if key.kind == KeyEventKind::Release {
        return Action::Ignore;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    // AltGr is reported as Ctrl+Alt on Windows and must type its character.
    let shortcut = ctrl && !alt;

    match key.code {
        KeyCode::Char(c) if shortcut => match c.to_ascii_lowercase() {
            'c' | 'q' => Action::Input(Input::Quit),
            'z' => Action::Input(Input::Undo),
            'y' => Action::Input(Input::Redo),
            'r' => Action::Input(Input::Reset),
            'n' => Action::Input(Input::New),
            't' => Action::Input(Input::ToggleTheme),
            'l' => Action::Input(Input::ToggleAnalysis),
            'g' => Action::Input(Input::Hint),
            'u' => Action::Input(Input::ClearLine),
            'w' => Action::Input(Input::DeleteWord),
            'a' => Action::Input(Input::Home),
            'e' => Action::Input(Input::End),
            _ => Action::Ignore,
        },
        KeyCode::Left if shortcut => Action::Scroll { dx: -4, dy: 0 },
        KeyCode::Right if shortcut => Action::Scroll { dx: 4, dy: 0 },
        KeyCode::Up if shortcut => Action::Scroll { dx: 0, dy: -2 },
        KeyCode::Down if shortcut => Action::Scroll { dx: 0, dy: 2 },
        _ if alt && !ctrl => Action::Ignore,
        KeyCode::Char(c) => Action::Input(Input::Char(c)),
        KeyCode::Enter => Action::Input(Input::Enter),
        KeyCode::Esc => Action::Input(Input::Escape),
        KeyCode::Backspace => Action::Input(Input::Backspace),
        KeyCode::Delete => Action::Input(Input::Delete),
        KeyCode::Left => Action::Input(Input::Left),
        KeyCode::Right => Action::Input(Input::Right),
        KeyCode::Home => Action::Input(Input::Home),
        KeyCode::End => Action::Input(Input::End),
        KeyCode::Up => Action::Input(Input::Up),
        KeyCode::Down => Action::Input(Input::Down),
        KeyCode::PageUp => Action::Input(Input::PageUp),
        KeyCode::PageDown => Action::Input(Input::PageDown),
        KeyCode::F(1) => Action::Input(Input::Help),
        KeyCode::F(2) => Action::Input(Input::Hint),
        KeyCode::F(3) => Action::Input(Input::Check),
        KeyCode::F(4) => Action::Input(Input::ToggleExam),
        KeyCode::F(5) => Action::Input(Input::CycleView),
        _ => Action::Ignore,
    }
}

/// Applies an action to the application and the view.
pub fn dispatch(action: Action, app: &mut App, view: &mut View) {
    let overlay_open = *app.overlay() != Overlay::None;
    match action {
        // Scrolling keys move the open overlay, or the matrix if none is open.
        Action::Input(input @ (Input::Up | Input::Down | Input::PageUp | Input::PageDown))
            if overlay_open =>
        {
            let dy = match input {
                Input::Up => -1,
                Input::Down => 1,
                Input::PageUp => -8,
                _ => 8,
            };
            view.scroll_overlay(dy);
        }
        Action::Input(input @ (Input::PageUp | Input::PageDown)) => {
            view.scroll(0, if input == Input::PageUp { -6 } else { 6 });
        }
        Action::Input(input) => {
            let before = std::mem::discriminant(app.overlay());
            app.handle(input);
            if std::mem::discriminant(app.overlay()) != before {
                view.overlay_scroll = 0;
            }
            // A different matrix may have a different size: start at the top left.
            if matches!(input, Input::New | Input::Enter) {
                view.scroll_x = 0;
                view.scroll_y = 0;
            }
        }
        Action::Scroll { dx, dy } => view.scroll(dx, dy),
        Action::Ignore => {}
    }
}

/// Runs the interactive session until the user quits.
///
/// Sets up the terminal (raw mode, alternate screen) and restores it on
/// exit and on panic.
///
/// # Errors
///
/// Propagates terminal I/O errors.
pub fn run(mut app: App, mut view: View) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app, &mut view);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, view: &mut View) -> io::Result<()> {
    let mut last = Instant::now();
    while !app.should_quit() {
        terminal.draw(|frame| ui::render(frame, app, view))?;
        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
        {
            dispatch(map_key(key), app, view);
        }
        let now = Instant::now();
        app.tick(now.duration_since(last));
        last = now;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppConfig;
    use crate::matrix::Matrix;
    use crate::ui::matrix_view::Brackets;
    use crate::ui::palette::ColorDepth;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn maps_control_shortcuts() {
        let ctrl = |c| map_key(key(KeyCode::Char(c), KeyModifiers::CONTROL));
        assert_eq!(ctrl('z'), Action::Input(Input::Undo));
        assert_eq!(ctrl('Y'), Action::Input(Input::Redo));
        assert_eq!(ctrl('r'), Action::Input(Input::Reset));
        assert_eq!(ctrl('n'), Action::Input(Input::New));
        assert_eq!(ctrl('t'), Action::Input(Input::ToggleTheme));
        assert_eq!(ctrl('l'), Action::Input(Input::ToggleAnalysis));
        assert_eq!(ctrl('q'), Action::Input(Input::Quit));
        assert_eq!(ctrl('c'), Action::Input(Input::Quit));
        assert_eq!(ctrl('x'), Action::Ignore);
    }

    #[test]
    fn plain_characters_are_typed_including_altgr() {
        assert_eq!(
            map_key(key(KeyCode::Char('m'), KeyModifiers::NONE)),
            Action::Input(Input::Char('m'))
        );
        assert_eq!(
            map_key(key(KeyCode::Char('M'), KeyModifiers::SHIFT)),
            Action::Input(Input::Char('M'))
        );
        let altgr = KeyModifiers::CONTROL | KeyModifiers::ALT;
        assert_eq!(
            map_key(key(KeyCode::Char('€'), altgr)),
            Action::Input(Input::Char('€'))
        );
        assert_eq!(
            map_key(key(KeyCode::Char('x'), KeyModifiers::ALT)),
            Action::Ignore
        );
    }

    #[test]
    fn maps_function_and_navigation_keys() {
        let plain = |code| map_key(key(code, KeyModifiers::NONE));
        assert_eq!(plain(KeyCode::F(1)), Action::Input(Input::Help));
        assert_eq!(plain(KeyCode::F(2)), Action::Input(Input::Hint));
        assert_eq!(plain(KeyCode::F(3)), Action::Input(Input::Check));
        assert_eq!(plain(KeyCode::F(4)), Action::Input(Input::ToggleExam));
        assert_eq!(plain(KeyCode::F(5)), Action::Input(Input::CycleView));
        assert_eq!(plain(KeyCode::Enter), Action::Input(Input::Enter));
        assert_eq!(plain(KeyCode::Up), Action::Input(Input::Up));
        assert_eq!(plain(KeyCode::F(9)), Action::Ignore);
        assert_eq!(plain(KeyCode::Tab), Action::Ignore);
    }

    #[test]
    fn control_arrows_scroll_the_matrix() {
        let c = |code| map_key(key(code, KeyModifiers::CONTROL));
        assert_eq!(c(KeyCode::Left), Action::Scroll { dx: -4, dy: 0 });
        assert_eq!(c(KeyCode::Down), Action::Scroll { dx: 0, dy: 2 });
    }

    #[test]
    fn key_releases_are_ignored() {
        let mut e = key(KeyCode::Char('a'), KeyModifiers::NONE);
        e.kind = KeyEventKind::Release;
        assert_eq!(map_key(e), Action::Ignore);
    }

    #[test]
    fn page_keys_scroll_the_matrix_unless_an_overlay_is_open() {
        let mut app = App::new(AppConfig {
            matrix: Some(Matrix::course_example()),
            ..AppConfig::default()
        });
        let mut view = View::new(ColorDepth::TrueColor, Brackets::Tall);
        dispatch(Action::Input(Input::PageDown), &mut app, &mut view);
        assert_eq!(view.scroll_y, 6);
        dispatch(Action::Input(Input::PageUp), &mut app, &mut view);
        dispatch(Action::Input(Input::PageUp), &mut app, &mut view);
        assert_eq!(view.scroll_y, 0);
        dispatch(Action::Input(Input::Help), &mut app, &mut view);
        dispatch(Action::Input(Input::PageDown), &mut app, &mut view);
        assert_eq!(view.scroll_y, 0);
        assert_eq!(view.overlay_scroll, 8);
        dispatch(Action::Input(Input::Up), &mut app, &mut view);
        assert_eq!(view.overlay_scroll, 7);
        // Closing the overlay resets its scroll position.
        dispatch(Action::Input(Input::Escape), &mut app, &mut view);
        assert_eq!(view.overlay_scroll, 0);
    }
}
