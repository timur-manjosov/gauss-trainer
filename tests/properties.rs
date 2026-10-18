//! Property-based tests (proptest).
//!
//! The mathematical properties are checked against *independent* oracles
//! where possible: the rank via minors, and the Treppennormalform via a
//! direct transcription of its definition.

use gauss_trainer::app::{App, AppConfig, Theme};
use gauss_trainer::check::check;
use gauss_trainer::echelon::{Analysis, is_rref, next_step, rank, rref, solve};
use gauss_trainer::generator::{Preset, Rng, generate};
use gauss_trainer::hint::hint;
use gauss_trainer::matrix::Matrix;
use gauss_trainer::ops::RowOperation;
use gauss_trainer::parse::{Command, NewSpec, parse_command, parse_matrix, parse_number};
use gauss_trainer::rational::{Q, int, ratio, to_text};
use gauss_trainer::ui::matrix_view::Brackets;
use gauss_trainer::ui::palette::ColorDepth;
use gauss_trainer::ui::{self, View};
use num_traits::{One, Zero};
use proptest::prelude::*;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ----- strategies --------------------------------------------------------

fn matrix_with_dims(rows: usize, cols: usize) -> impl Strategy<Value = Matrix> {
    prop::collection::vec(-6i64..=6, rows * cols).prop_map(move |v| {
        Matrix::new(rows, cols, v.into_iter().map(int).collect()).expect("valid dimensions")
    })
}

fn any_matrix(max_dim: usize) -> impl Strategy<Value = Matrix> {
    (1..=max_dim, 1..=max_dim).prop_flat_map(|(r, c)| matrix_with_dims(r, c))
}

/// Raw material for one operation; `build_op` turns it into a valid one.
type OpSeed = (u8, usize, usize, i64, i64);

fn op_seed() -> impl Strategy<Value = OpSeed> {
    (0u8..3, 0usize..6, 0usize..6, -6i64..=6, 1i64..=5)
}

/// Builds a legal operation for a matrix with `rows` rows from arbitrary numbers.
fn build_op(rows: usize, (kind, a, b, n, d): OpSeed) -> RowOperation {
    let first = a % rows;
    let other = (first + 1 + b % rows.saturating_sub(1).max(1)) % rows;
    let factor = ratio(n, d).expect("denominator is positive");
    match kind {
        1 if rows > 1 => RowOperation::add_multiple(factor, first, other).expect("distinct rows"),
        2 if rows > 1 => RowOperation::swap(first, other).expect("distinct rows"),
        _ => {
            let nonzero = if factor.is_zero() { Q::one() } else { factor };
            RowOperation::scale(first, nonzero).expect("nonzero factor")
        }
    }
}

fn op_seeds(max_len: usize) -> impl Strategy<Value = Vec<OpSeed>> {
    prop::collection::vec(op_seed(), 0..=max_len)
}

fn apply_all(m: &Matrix, seeds: &[OpSeed]) -> Matrix {
    let mut current = m.clone();
    for &seed in seeds {
        build_op(m.rows(), seed)
            .apply(&mut current)
            .expect("legal operation");
    }
    current
}

// ----- oracles -----------------------------------------------------------

fn det(rows: &[Vec<Q>]) -> Q {
    match rows.len() {
        0 => Q::one(),
        1 => rows[0][0].clone(),
        n => (0..n)
            .map(|j| {
                let minor: Vec<Vec<Q>> = rows[1..]
                    .iter()
                    .map(|r| {
                        r.iter()
                            .enumerate()
                            .filter(|&(k, _)| k != j)
                            .map(|(_, q)| q.clone())
                            .collect()
                    })
                    .collect();
                let term = &rows[0][j] * det(&minor);
                if j % 2 == 0 { term } else { -term }
            })
            .fold(Q::zero(), |acc, t| acc + t),
    }
}

fn subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn go(start: usize, n: usize, k: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if current.len() == k {
            out.push(current.clone());
            return;
        }
        for i in start..n {
            current.push(i);
            go(i + 1, n, k, current, out);
            current.pop();
        }
    }
    let mut out = Vec::new();
    go(0, n, k, &mut Vec::new(), &mut out);
    out
}

/// Rank as the size of the largest nonvanishing minor.
fn rank_by_minors(m: &Matrix) -> usize {
    for k in (1..=m.rows().min(m.cols())).rev() {
        for rows in subsets(m.rows(), k) {
            for cols in subsets(m.cols(), k) {
                let sub: Vec<Vec<Q>> = rows
                    .iter()
                    .map(|&i| cols.iter().map(|&j| m[(i, j)].clone()).collect())
                    .collect();
                if !det(&sub).is_zero() {
                    return k;
                }
            }
        }
    }
    0
}

/// The definition of Treppennormalform, transcribed literally.
fn is_rref_direct(m: &Matrix) -> bool {
    let mut previous_pivot: Option<usize> = None;
    let mut seen_zero_row = false;
    for i in 0..m.rows() {
        match (0..m.cols()).find(|&j| !m[(i, j)].is_zero()) {
            None => seen_zero_row = true,
            Some(p) => {
                if seen_zero_row || !m[(i, p)].is_one() || previous_pivot.is_some_and(|l| p <= l) {
                    return false;
                }
                previous_pivot = Some(p);
                if (0..m.rows()).any(|k| k != i && !m[(k, p)].is_zero()) {
                    return false;
                }
            }
        }
    }
    true
}

// ----- operations --------------------------------------------------------

proptest! {
    #[test]
    fn every_operation_is_undone_by_its_inverse(m in any_matrix(6), seed in op_seed()) {
        let op = build_op(m.rows(), seed);
        let mut work = m.clone();
        op.apply(&mut work).unwrap();
        op.inverse().apply(&mut work).unwrap();
        prop_assert_eq!(&work, &m);
        prop_assert_eq!(op.inverse().inverse(), op);
    }

    #[test]
    fn row_operations_preserve_the_rank(m in any_matrix(5), seeds in op_seeds(8)) {
        let after = apply_all(&m, &seeds);
        prop_assert_eq!(rank(&after), rank(&m));
    }

    #[test]
    fn rank_agrees_with_the_minor_oracle(m in any_matrix(4)) {
        prop_assert_eq!(rank(&m), rank_by_minors(&m));
        prop_assert_eq!(Analysis::new(&m).rank(), rank_by_minors(&m));
    }

    #[test]
    fn failed_operations_never_modify_the_matrix(m in any_matrix(4), extra in 0usize..4) {
        let bad = RowOperation::Swap { first: 0, second: m.rows() + extra };
        let mut work = m.clone();
        prop_assert!(bad.apply(&mut work).is_err());
        prop_assert_eq!(work, m);
    }

    #[test]
    fn descriptions_and_commands_round_trip(m in any_matrix(6), seed in op_seed()) {
        let op = build_op(m.rows(), seed);
        prop_assert_eq!(parse_command(&op.to_command()).unwrap(), Command::Operation(op.clone()));
        prop_assert!(op.describe().ends_with('.'));
    }
}

// ----- the reference solver and the Treppennormalform ------------------------

proptest! {
    #[test]
    fn the_solver_always_ends_in_treppennormalform(m in any_matrix(6)) {
        let solution = solve(&m);
        prop_assert!(is_rref_direct(&solution.result), "{}", solution.result);
        prop_assert!(is_rref(&solution.result));
        // Replaying the steps reproduces the result, and every step is legal.
        let mut replay = m.clone();
        for step in &solution.steps {
            prop_assert!(step.op.apply(&mut replay).is_ok());
        }
        prop_assert_eq!(&replay, &solution.result);
        prop_assert_eq!(rref(&m), solution.result);
    }

    #[test]
    fn the_analysis_agrees_with_the_definition(m in any_matrix(5)) {
        prop_assert_eq!(is_rref(&m), is_rref_direct(&m));
        prop_assert_eq!(is_rref(&m), rref(&m) == m);
        prop_assert_eq!(is_rref(&m), solve(&m).steps.is_empty());
        prop_assert_eq!(Analysis::new(&m).missing(false).is_empty(), is_rref(&m));
    }

    #[test]
    fn the_treppennormalform_is_unique(m in any_matrix(5), a in op_seeds(12), b in op_seeds(12)) {
        let via_a = apply_all(&m, &a);
        let via_b = apply_all(&m, &b);
        prop_assert_eq!(rref(&via_a), rref(&m));
        prop_assert_eq!(rref(&via_a), rref(&via_b));
    }

    #[test]
    fn the_rank_equals_the_number_of_nonzero_rows_of_the_normal_form(m in any_matrix(6)) {
        let r = rref(&m);
        let nonzero = (0..r.rows()).filter(|&i| !r.row_is_zero(i)).count();
        prop_assert_eq!(rank(&m), nonzero);
        prop_assert_eq!(Analysis::new(&r).pivots().len(), nonzero);
    }

    #[test]
    fn solver_steps_are_bounded(m in any_matrix(6)) {
        let steps = solve(&m).steps.len();
        // per pivot: at most one swap, one scale and rows-1 eliminations
        prop_assert!(steps <= m.rows().min(m.cols()) * (m.rows() + 1));
    }

    #[test]
    fn the_hint_names_the_solvers_next_step(m in any_matrix(5)) {
        let level_three = hint(3, &m).text;
        match next_step(&m) {
            Some(step) => prop_assert!(level_three.contains(&step.op.to_command())),
            None => prop_assert!(level_three.contains("Treppennormalform")),
        }
    }

    #[test]
    fn check_recognises_the_current_matrix_and_the_normal_form(m in any_matrix(5)) {
        let report = check(&m, &m);
        prop_assert!(report.matches_current && report.diff_current.is_empty());
        let normal = rref(&m);
        let report = check(&normal, &m);
        prop_assert!(report.matches_rref && report.diff_rref.is_empty());
    }
}

// ----- parsing ---------------------------------------------------------

proptest! {
    #[test]
    fn rationals_round_trip_through_text(n in -10_000i64..=10_000, d in 1i64..=10_000) {
        let q = ratio(n, d).unwrap();
        prop_assert_eq!(parse_number(&to_text(&q)).unwrap(), q);
    }

    #[test]
    fn decimals_are_converted_exactly(whole in 0u32..1000, frac in 0u32..10_000, digits in 1usize..=4, negative: bool) {
        let frac = frac % 10u32.pow(digits as u32);
        let text = format!("{}{whole}.{frac:0digits$}", if negative { "-" } else { "" });
        let scale = 10i64.pow(digits as u32);
        let expected = ratio(i64::from(whole) * scale + i64::from(frac), scale).unwrap();
        let expected = if negative { -expected } else { expected };
        prop_assert_eq!(parse_number(&text).unwrap(), expected.clone());
        prop_assert_eq!(parse_number(&text.replace('.', ",")).unwrap(), expected);
    }

    #[test]
    fn matrices_round_trip_through_their_text(m in any_matrix(6)) {
        prop_assert_eq!(parse_matrix(&m.to_string()).unwrap(), m);
    }

    #[test]
    fn parsers_never_panic_on_arbitrary_text(s in ".{0,60}") {
        let _ = parse_number(&s);
        let _ = parse_matrix(&s);
        let _ = parse_command(&s);
    }

    #[test]
    fn parsers_never_panic_on_plausible_text(s in "[0-9a-z/.,;\\[\\] −+-]{0,40}") {
        let _ = parse_number(&s);
        let _ = parse_matrix(&s);
        let _ = parse_command(&s);
        let _ = parse_command(&format!("m {s}"));
        let _ = parse_command(&format!("a {s}"));
        let _ = parse_command(&format!("s {s}"));
        let _ = parse_command(&format!("n {s}"));
    }
}

// ----- generator -------------------------------------------------------

fn preset_strategy() -> impl Strategy<Value = Preset> {
    prop::sample::select(Preset::ALL.to_vec())
}

proptest! {
    #[test]
    fn generated_matrices_keep_their_promises(seed: u64, rows in 0usize..9, cols in 0usize..9, preset in preset_strategy()) {
        let m = generate(&mut Rng::new(seed), rows, cols, preset).unwrap();
        let (r, c) = preset.fit_dims(rows, cols);
        prop_assert_eq!((m.rows(), m.cols()), (r, c));
        prop_assert!((1..=6).contains(&m.rows()) && (1..=6).contains(&m.cols()));
        prop_assert!(rank(&m) > 0 && !is_rref(&m));
        match preset {
            Preset::Generic => {}
            Preset::ZeroTopLeft => prop_assert!(m[(0, 0)].is_zero()),
            Preset::DependentRow => prop_assert!(rank(&m) < m.rows()),
            Preset::MoreColumns => prop_assert!(m.cols() > m.rows()),
            Preset::MoreRows => prop_assert!(m.rows() > m.cols()),
        }
        // Equal seeds reproduce the matrix.
        prop_assert_eq!(generate(&mut Rng::new(seed), rows, cols, preset).unwrap(), m);
    }
}

// ----- the application state machine -----------------------------------

#[derive(Debug, Clone)]
enum Step {
    Op(OpSeed),
    Undo,
    Redo,
    Reset,
}

fn step_strategy() -> impl Strategy<Value = Step> {
    prop_oneof![
        6 => op_seed().prop_map(Step::Op),
        2 => Just(Step::Undo),
        2 => Just(Step::Redo),
        1 => Just(Step::Reset),
    ]
}

proptest! {
    #[test]
    fn the_history_stays_consistent(m in any_matrix(5), steps in prop::collection::vec(step_strategy(), 0..40)) {
        let mut app = App::new(AppConfig { matrix: Some(m.clone()), ..AppConfig::default() });
        for step in steps {
            match step {
                Step::Op(seed) => app.execute(Command::Operation(build_op(m.rows(), seed))),
                Step::Undo => app.execute(Command::Undo),
                Step::Redo => app.execute(Command::Redo),
                Step::Reset => app.execute(Command::Reset),
            }
            // The current matrix is A₀ with the executed steps applied.
            let executed: Vec<_> = app.protocol().into_iter().filter(|e| !e.undone).collect();
            prop_assert_eq!(executed.len(), app.step());
            let mut replay = m.clone();
            for entry in &executed {
                entry.op.apply(&mut replay).unwrap();
            }
            prop_assert_eq!(app.matrix(), &replay);
            // Row operations never change the rank or the normal form.
            prop_assert_eq!(app.analysis().rank(), rank(&m));
            prop_assert_eq!(rref(app.matrix()), rref(&m));
            prop_assert_eq!(app.initial(), &m);
        }
        // Undoing everything leads back to A₀; redoing everything replays the whole protocol.
        let all_steps: Vec<RowOperation> = app.protocol().into_iter().map(|e| e.op.clone()).collect();
        for _ in 0..all_steps.len() + 1 { app.execute(Command::Undo); }
        prop_assert_eq!(app.matrix(), &m);
        for _ in 0..all_steps.len() + 1 { app.execute(Command::Redo); }
        let mut expected = m.clone();
        for op in &all_steps { op.apply(&mut expected).unwrap(); }
        prop_assert_eq!(app.matrix(), &expected);
    }

    #[test]
    fn new_matrices_are_reproducible_through_the_seed(seed: u64, rows in 1usize..=6, cols in 1usize..=6, preset in preset_strategy()) {
        let spec = NewSpec { size: Some((rows, cols)), preset: Some(preset), seed: Some(seed) };
        let mut a = App::new(AppConfig::default());
        let mut b = App::new(AppConfig { seed: 12345, ..AppConfig::default() });
        a.execute(Command::New(spec.clone()));
        b.execute(Command::New(spec));
        prop_assert_eq!(a.matrix(), b.matrix());
        prop_assert_eq!(a.seed(), Some(seed));
    }
}

// ----- rendering never panics -------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn rendering_survives_any_size_and_state(
        m in any_matrix(6),
        seeds in op_seeds(6),
        width in 1u16..=170,
        height in 1u16..=60,
        light: bool,
        exam: bool,
        stacked in 0u8..3,
        overlay in 0u8..5,
        indexed: bool,
    ) {
        let mut app = App::new(AppConfig { matrix: Some(m.clone()), ..AppConfig::default() });
        for seed in seeds {
            app.execute(Command::Operation(build_op(m.rows(), seed)));
        }
        if light && app.theme() != Theme::Light { app.execute(Command::ToggleTheme); }
        for _ in 0..stacked { app.execute(Command::CycleView); }
        if exam { app.execute(Command::ToggleExam); }
        match overlay {
            1 => app.execute(Command::Help),
            2 => app.execute(Command::Solution),
            3 => app.execute(Command::Check(Some(m.clone()))),
            4 => { app.execute(Command::Hint); }
            _ => {}
        }
        let depth = if indexed { ColorDepth::Indexed256 } else { ColorDepth::TrueColor };
        let mut view = View::new(depth, Brackets::Tall);
        view.scroll(1000, 1000);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| ui::render(frame, &app, &mut view)).unwrap();
        terminal.draw(|frame| ui::render(frame, &app, &mut view)).unwrap();
    }
}
