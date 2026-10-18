//! End-to-end: type the course example through the real key-handling path.

use gauss_trainer::app::{App, AppConfig};
use gauss_trainer::matrix::Matrix;
use gauss_trainer::parse::parse_matrix;
use gauss_trainer::tui::{dispatch, map_key};
use gauss_trainer::ui::matrix_view::Brackets;
use gauss_trainer::ui::palette::ColorDepth;
use gauss_trainer::ui::{self, View};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

struct Session {
    app: App,
    view: View,
}

impl Session {
    fn new() -> Self {
        let config = AppConfig {
            matrix: Some(Matrix::course_example()),
            ..AppConfig::default()
        };
        Self {
            app: App::new(config),
            view: View::new(ColorDepth::TrueColor, Brackets::Tall),
        }
    }

    fn press(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        dispatch(
            map_key(KeyEvent::new(code, modifiers)),
            &mut self.app,
            &mut self.view,
        );
    }

    fn type_line(&mut self, line: &str) {
        for c in line.chars() {
            self.press(KeyCode::Char(c), KeyModifiers::NONE);
        }
        self.press(KeyCode::Enter, KeyModifiers::NONE);
    }

    fn screen(&mut self) -> String {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal
            .draw(|f| ui::render(f, &self.app, &mut self.view))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        buffer
            .content()
            .chunks(100)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[test]
fn solving_the_example_by_hand() {
    let mut s = Session::new();
    for line in [
        "a -2 1 2",
        "a -3 1 3",
        "m 2 1/3",
        "a 2 2 1",
        "a -7 2 3",
        "m 3 3/7",
        "a -1/3 3 1",
        "a 4/3 3 2",
    ] {
        s.type_line(line);
        assert!(
            s.app
                .feedback()
                .is_none_or(|f| f.tone != gauss_trainer::app::Tone::Error),
            "{line}"
        );
    }
    let expected = parse_matrix("[[1,0,0,15/7],[0,1,0,-4/7],[0,0,1,-10/7]]").unwrap();
    assert_eq!(s.app.matrix(), &expected);
    assert_eq!(s.app.step(), 8);
    assert!(s.app.analysis().is_rref());

    let screen = s.screen();
    assert!(screen.contains("Treppennormalform erreicht ✓"), "{screen}");
    assert!(screen.contains("Rang r = 3"));
    assert!(screen.contains("Matrix · A₈"));
    assert!(screen.contains("A₇ → A₈"));
    let last = s.app.protocol().pop().unwrap();
    assert_eq!(
        last.op.describe(),
        "Das 4/3-fache von Zeile 3 wird zu Zeile 2 addiert."
    );
    assert!(screen.contains("Das 4/3-fache"), "{screen}");
    assert!(screen.contains("15") && screen.contains("10"), "{screen}");
}

#[test]
fn undo_redo_and_reset_via_shortcuts() {
    let mut s = Session::new();
    s.type_line("a -2 1 2");
    s.type_line("s 2 3");
    let after_two = s.app.matrix().clone();
    s.press(KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(s.app.step(), 1);
    s.press(KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert_eq!(s.app.matrix(), &after_two);
    s.press(KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert_eq!(s.app.matrix(), s.app.initial());
    assert!(
        s.screen().contains("A₀ → A₁"),
        "undone steps stay visible in the protocol"
    );
    s.press(KeyCode::Char('y'), KeyModifiers::CONTROL);
    s.press(KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert_eq!(s.app.matrix(), &after_two);
}

#[test]
fn typing_mistakes_are_corrected_in_the_command_bar() {
    let mut s = Session::new();
    for c in "a -2 1 3".chars() {
        s.press(KeyCode::Char(c), KeyModifiers::NONE);
    }
    s.press(KeyCode::Backspace, KeyModifiers::NONE);
    s.press(KeyCode::Char('2'), KeyModifiers::NONE);
    s.press(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(s.app.step(), 1);
    assert_eq!(
        s.app.protocol()[0].op.describe(),
        "Das (-2)-fache von Zeile 1 wird zu Zeile 2 addiert."
    );
    // History recall with the arrow keys.
    s.press(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(s.app.editor().text(), "a -2 1 2");
}

#[test]
fn hints_only_appear_on_request() {
    let mut s = Session::new();
    s.type_line("s 1 2");
    let quiet = s.screen();
    assert!(!quiet.contains("Tipp"), "{quiet}");
    s.press(KeyCode::F(2), KeyModifiers::NONE);
    assert!(s.screen().contains("Tipp 1/3"));
    s.press(KeyCode::F(2), KeyModifiers::NONE);
    s.press(KeyCode::F(2), KeyModifiers::NONE);
    assert!(s.screen().contains("Gib ein:"));
}

#[test]
fn quitting_with_ctrl_q_and_ctrl_c() {
    for key in ['q', 'c'] {
        let mut s = Session::new();
        s.press(KeyCode::Char(key), KeyModifiers::CONTROL);
        assert!(s.app.should_quit());
    }
}
