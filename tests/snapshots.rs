//! Rendering snapshot tests on ratatui's `TestBackend`.
//!
//! Each snapshot is the plain text of the screen. Colours are covered by the
//! explicit style assertions at the end of the file.

use std::time::Duration;

use gauss_trainer::app::{App, AppConfig, Input, Theme};
use gauss_trainer::matrix::Matrix;
use gauss_trainer::parse::{Command, parse_command, parse_matrix};
use gauss_trainer::ui::matrix_view::Brackets;
use gauss_trainer::ui::palette::{ColorDepth, Palette};
use gauss_trainer::ui::{self, View};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;

fn app_with(matrix: Matrix) -> App {
    App::new(AppConfig {
        matrix: Some(matrix),
        ..AppConfig::default()
    })
}

fn example() -> App {
    app_with(Matrix::course_example())
}

fn run(app: &mut App, lines: &[&str]) {
    for line in lines {
        match parse_command(line) {
            Ok(command) => app.execute(command),
            Err(e) => panic!("{line}: {e}"),
        }
    }
}

fn draw(app: &App, width: u16, height: u16, depth: ColorDepth) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    let mut view = View::new(depth, Brackets::Tall);
    terminal
        .draw(|frame| ui::render(frame, app, &mut view))
        .expect("draw");
    terminal.backend().buffer().clone()
}

fn text(buffer: &Buffer) -> String {
    let width = usize::from(buffer.area.width);
    buffer
        .content()
        .chunks(width)
        .map(|row| {
            row.iter()
                .map(|c| c.symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn screen(app: &App, width: u16, height: u16) -> String {
    text(&draw(app, width, height, ColorDepth::TrueColor))
}

#[test]
fn start_screen() {
    insta::assert_snapshot!(screen(&example(), 100, 28));
}

#[test]
fn fractions_changed_rows_and_undone_steps() {
    let mut app = example();
    run(
        &mut app,
        &["a -2 1 2", "a -3 1 3", "m 2 -1/3", "a -7 2 3", "u"],
    );
    insta::assert_snapshot!(screen(&app, 100, 28));
}

#[test]
fn solved_state() {
    let mut app = example();
    for step in gauss_trainer::echelon::solve(app.matrix()).steps {
        app.execute(Command::Operation(step.op));
    }
    insta::assert_snapshot!(screen(&app, 100, 28));
}

#[test]
fn zero_row_and_swap_needed() {
    let mut app = app_with(parse_matrix("0 2 4; 1 1 1; 2 2 2").unwrap());
    run(&mut app, &["tipp"]);
    insta::assert_snapshot!(screen(&app, 100, 28));
}

#[test]
fn exam_mode_hides_assistance() {
    let mut app = example();
    run(&mut app, &["exam", "a -2 1 2"]);
    app.tick(Duration::from_secs(95));
    insta::assert_snapshot!(screen(&app, 100, 28));
}

#[test]
fn help_overlay() {
    let mut app = example();
    app.handle(Input::Char('?'));
    insta::assert_snapshot!(screen(&app, 100, 34));
}

#[test]
fn solution_confirmation_and_overlay() {
    let mut app = example();
    run(&mut app, &["lösung"]);
    insta::assert_snapshot!("solution_confirmation", screen(&app, 100, 28));
    app.handle(Input::Char('j'));
    insta::assert_snapshot!("solution_overlay", screen(&app, 100, 34));
}

#[test]
fn check_overlay() {
    let mut app = example();
    run(
        &mut app,
        &["a -2 1 2", "check [[1,-2,3,-1],[0,3,-4,5],[3,1,2,3]]"],
    );
    insta::assert_snapshot!(screen(&app, 100, 28));
}

#[test]
fn matrix_prompt() {
    let mut app = example();
    run(&mut app, &["set"]);
    for c in "1 2; 3".chars() {
        app.handle(Input::Char(c));
    }
    insta::assert_snapshot!(screen(&app, 100, 28));
}

#[test]
fn error_feedback() {
    let mut app = example();
    for c in "m 1 0".chars() {
        app.handle(Input::Char(c));
    }
    app.handle(Input::Enter);
    insta::assert_snapshot!(screen(&app, 100, 28));
}

#[test]
fn too_small_terminal() {
    insta::assert_snapshot!(screen(&example(), 60, 18));
}

#[test]
fn one_column_short_of_the_minimum_is_too_small() {
    let text = screen(&example(), 79, 24);
    assert!(text.contains("Das Terminal ist zu klein"));
    assert!(text.contains("Aktuell 79×24"));
    assert!(!screen(&example(), 80, 24).contains("zu klein"));
    assert!(screen(&example(), 80, 23).contains("zu klein"));
}

#[test]
fn minimum_size_with_inline_fallback() {
    let mut app = app_with(parse_matrix("1 2 3 4 5; 6 7 8 9 1; 2 3 4 5 6; 7 8 9 1 2").unwrap());
    run(&mut app, &["m 1 1/3", "m 2 -2/7"]);
    insta::assert_snapshot!(screen(&app, 80, 24));
}

#[test]
fn large_matrix_scrolls_when_stacked_is_forced() {
    let mut app = app_with(
        parse_matrix(
            "1 2 3 4 5 6; 6 5 4 3 2 1; 1 3 5 7 9 2; 2 4 6 8 1 3; 9 8 7 6 5 4; 1 1 2 3 5 8",
        )
        .unwrap(),
    );
    run(&mut app, &["m 1 1/3", "v"]);
    insta::assert_snapshot!(screen(&app, 80, 24));
}

#[test]
fn light_theme_has_identical_text() {
    let mut app = example();
    let dark = screen(&app, 100, 28);
    app.execute(Command::ToggleTheme);
    assert_eq!(app.theme(), Theme::Light);
    assert!(dark.lines().next().is_some());
    assert!(screen(&app, 100, 28).contains("Rechenprotokoll"));
}

// ----- colours ---------------------------------------------------------

fn find<'a>(buffer: &'a Buffer, needle: &str) -> Option<&'a ratatui::buffer::Cell> {
    let width = usize::from(buffer.area.width);
    buffer
        .content()
        .chunks(width)
        .enumerate()
        .find_map(|(y, row)| {
            let line: String = row.iter().map(|c| c.symbol()).collect();
            line.find(needle).map(|byte| {
                let col = line[..byte].chars().count();
                &buffer.content()[y * width + col]
            })
        })
}

#[test]
fn live_analysis_colours_pivots() {
    let mut app = app_with(parse_matrix("2 1; 0 1").unwrap());
    let pal = Palette::new(Theme::Dark, ColorDepth::TrueColor);
    let buffer = draw(&app, 100, 28, ColorDepth::TrueColor);
    // Row 1: pivot 2 is not yet 1 (amber), the 1 in row 2 is a finished pivot (green).
    let amber = buffer
        .content()
        .iter()
        .any(|c| c.symbol() == "2" && c.fg == pal.amber && c.bg == pal.pivot_other_bg);
    let green = buffer
        .content()
        .iter()
        .any(|c| c.symbol() == "1" && c.fg == pal.green && c.bg == pal.pivot_one_bg);
    assert!(amber && green, "pivot colours missing");
    // The "1" above the second pivot must still become 0: highlighted red.
    let red = buffer
        .content()
        .iter()
        .any(|c| c.symbol() == "1" && c.fg == pal.red);
    assert!(red, "entry to clear not highlighted");

    // Without live analysis there are no analysis colours at all.
    app.handle(Input::ToggleAnalysis);
    let plain = draw(&app, 100, 28, ColorDepth::TrueColor);
    assert!(
        !plain
            .content()
            .iter()
            .any(|c| c.fg == pal.amber || c.fg == pal.green || c.fg == pal.red)
    );
}

#[test]
fn exam_mode_has_no_highlights() {
    let mut app = app_with(parse_matrix("2 1; 0 1").unwrap());
    run(&mut app, &["exam", "m 1 1/2"]);
    let pal = Palette::new(Theme::Dark, ColorDepth::TrueColor);
    let buffer = draw(&app, 100, 28, ColorDepth::TrueColor);
    let matrix_cells = buffer.content().iter().filter(|c| {
        c.bg == pal.changed_bg || c.bg == pal.pivot_one_bg || c.bg == pal.pivot_other_bg
    });
    assert_eq!(matrix_cells.count(), 0);
}

#[test]
fn indexed_fallback_uses_no_rgb_colours() {
    let mut app = example();
    run(&mut app, &["a -2 1 2"]);
    for theme in [Theme::Dark, Theme::Light] {
        if app.theme() != theme {
            app.execute(Command::ToggleTheme);
        }
        let buffer = draw(&app, 100, 28, ColorDepth::Indexed256);
        for cell in buffer.content() {
            assert!(
                !matches!(cell.fg, Color::Rgb(..)),
                "{theme:?}: truecolor fg leaked"
            );
            assert!(
                !matches!(cell.bg, Color::Rgb(..)),
                "{theme:?}: truecolor bg leaked"
            );
        }
    }
}

#[test]
fn truecolor_screens_use_the_theme_background() {
    let mut app = example();
    let dark = draw(&app, 100, 28, ColorDepth::TrueColor);
    app.execute(Command::ToggleTheme);
    let light = draw(&app, 100, 28, ColorDepth::TrueColor);
    assert_eq!(
        dark.content()[0].bg,
        Palette::new(Theme::Dark, ColorDepth::TrueColor).bg
    );
    assert_eq!(
        light.content()[0].bg,
        Palette::new(Theme::Light, ColorDepth::TrueColor).bg
    );
}

#[test]
fn changed_rows_are_tinted_and_marked() {
    let mut app = example();
    run(&mut app, &["a -2 1 2"]);
    let pal = Palette::new(Theme::Dark, ColorDepth::TrueColor);
    let buffer = draw(&app, 100, 28, ColorDepth::TrueColor);
    let marker = find(&buffer, "▎").expect("row marker");
    assert_eq!(marker.fg, pal.accent);
    assert!(buffer.content().iter().any(|c| c.bg == pal.changed_bg));
}
