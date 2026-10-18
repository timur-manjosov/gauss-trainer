//! The application state machine.
//!
//! [`App`] owns everything that changes while the student works: the matrix,
//! the undo/redo history, the toggles, the command line and the open overlay.
//! It consumes abstract [`Input`]s and parsed [`Command`]s and performs no
//! I/O and no rendering; the `ui` module only reads its state.
//!
//! # History model
//!
//! `done` holds the operations leading from `A₀` to the current matrix `Aₖ`
//! (`k = done.len()`), `undone` is a stack of steps that can be redone. A new
//! operation clears `undone`. `reset` is "undo everything" and therefore
//! itself reversible with redo. The current matrix is always recomputed by
//! replaying `done` on `A₀`, so undo can never drift from the history.

use std::time::Duration;

use crate::check::{CheckReport, check};
use crate::echelon::{Analysis, Solution, rref, solve};
use crate::generator::{DEFAULT_COLS, DEFAULT_ROWS, Preset, Rng, generate};
use crate::hint::{MAX_LEVEL, hint};
use crate::input::{CommandHistory, LineEditor};
use crate::matrix::Matrix;
use crate::notation::matrix_label;
use crate::ops::RowOperation;
use crate::parse::{Command, NewSpec, parse_command, parse_matrix};

/// Colour theme of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Light text on a dark background.
    #[default]
    Dark,
    /// Dark text on a light background.
    Light,
}

impl Theme {
    /// The other theme.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    /// German name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "dunkel",
            Self::Light => "hell",
        }
    }
}

/// How matrix entries are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatrixLayout {
    /// Stacked fractions when there is room, inline `a/b` otherwise.
    #[default]
    Auto,
    /// Always stacked fractions (the matrix panel scrolls if needed).
    Stacked,
    /// Always inline fractions, one line per row.
    Inline,
}

impl MatrixLayout {
    /// The next layout in the cycle.
    #[must_use]
    pub fn cycled(self) -> Self {
        match self {
            Self::Auto => Self::Stacked,
            Self::Stacked => Self::Inline,
            Self::Inline => Self::Auto,
        }
    }

    /// German name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "automatisch",
            Self::Stacked => "gestapelte Brüche",
            Self::Inline => "Brüche in einer Zeile",
        }
    }
}

/// The tone of a [`Feedback`] message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Neutral information.
    Info,
    /// Something worked or was achieved.
    Success,
    /// A hint requested by the student.
    Hint,
    /// Input was rejected.
    Error,
}

/// A message shown below the matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    /// How to present the message.
    pub tone: Tone,
    /// The German message text.
    pub text: String,
}

/// What the command bar currently asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Prompt {
    /// A command such as `a -2 1 2`.
    #[default]
    Command,
    /// A matrix that replaces the current exercise.
    NewMatrix,
    /// A hand-computed matrix to check.
    CheckMatrix,
}

/// A modal panel on top of the main view.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Overlay {
    /// No overlay.
    #[default]
    None,
    /// The help screen.
    Help,
    /// "Really show the reference solution?"
    ConfirmSolution,
    /// The reference solution of the current matrix.
    Solution {
        /// Index of the matrix the solution starts from.
        from_step: usize,
        /// The solution.
        solution: Box<Solution>,
    },
    /// The result of check mode.
    Check {
        /// Index of the matrix the entry was compared with.
        at_step: usize,
        /// The comparison.
        report: Box<CheckReport>,
    },
}

/// Abstract user input, independent of any terminal library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// A typed character.
    Char(char),
    /// Delete the character before the cursor.
    Backspace,
    /// Delete the character under the cursor.
    Delete,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
    /// Cursor to the start of the line.
    Home,
    /// Cursor to the end of the line.
    End,
    /// Submit the line.
    Enter,
    /// Cancel / clear.
    Escape,
    /// Older entry in the command history (scrolls overlays, see the `tui` module).
    Up,
    /// Newer entry in the command history (scrolls overlays, see the `tui` module).
    Down,
    /// Scroll a page up inside overlays.
    PageUp,
    /// Scroll a page down inside overlays.
    PageDown,
    /// Clear the whole line.
    ClearLine,
    /// Delete the word before the cursor.
    DeleteWord,
    /// Undo shortcut.
    Undo,
    /// Redo shortcut.
    Redo,
    /// Reset shortcut.
    Reset,
    /// New-matrix shortcut.
    New,
    /// Theme shortcut.
    ToggleTheme,
    /// Live-analysis shortcut.
    ToggleAnalysis,
    /// Exam-mode shortcut.
    ToggleExam,
    /// Layout shortcut.
    CycleView,
    /// Help shortcut.
    Help,
    /// Hint shortcut.
    Hint,
    /// Check-mode shortcut.
    Check,
    /// Quit shortcut.
    Quit,
}

/// Starting configuration of an [`App`].
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Seed of the first matrix (and of the session's sequence of matrices).
    pub seed: u64,
    /// Requested number of rows.
    pub rows: usize,
    /// Requested number of columns.
    pub cols: usize,
    /// Exercise type.
    pub preset: Preset,
    /// A user-supplied matrix; overrides generation.
    pub matrix: Option<Matrix>,
    /// Initial theme.
    pub theme: Theme,
    /// Whether the live analysis starts switched on.
    pub live_analysis: bool,
    /// Whether to start directly in exam mode.
    pub exam: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
            preset: Preset::Generic,
            matrix: None,
            theme: Theme::Dark,
            live_analysis: true,
            exam: false,
        }
    }
}

/// One line of the Rechenprotokoll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolEntry<'a> {
    /// Index of the matrix before the step (`k` in `Aₖ → Aₖ₊₁`).
    pub from: usize,
    /// Index of the matrix after the step.
    pub to: usize,
    /// The operation.
    pub op: &'a RowOperation,
    /// `true` if the step was undone and could be redone.
    pub undone: bool,
}

/// Formats a duration as `mm:ss`.
#[must_use]
pub fn format_clock(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// The application state.
#[derive(Debug)]
pub struct App {
    rows: usize,
    cols: usize,
    preset: Preset,
    rng: Rng,
    seed: Option<u64>,
    initial: Matrix,
    current: Matrix,
    analysis: Analysis,
    done: Vec<RowOperation>,
    undone: Vec<RowOperation>,
    changed: Vec<usize>,
    live: bool,
    exam: Option<Duration>,
    theme: Theme,
    layout: MatrixLayout,
    hint_level: u8,
    prompt: Prompt,
    editor: LineEditor,
    command_history: CommandHistory,
    matrix_history: CommandHistory,
    feedback: Option<Feedback>,
    overlay: Overlay,
    quit: bool,
}

impl App {
    /// Creates the application and its first exercise.
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        let (rows, cols) = config.preset.fit_dims(config.rows, config.cols);
        let (initial, seed) = match config.matrix {
            Some(m) => (m, None),
            None => {
                let generated = generate(&mut Rng::new(config.seed), rows, cols, config.preset);
                (
                    generated.unwrap_or_else(|_| Matrix::course_example()),
                    Some(config.seed),
                )
            }
        };
        let mut app = Self {
            rows,
            cols,
            preset: config.preset,
            rng: Rng::new(config.seed),
            seed,
            analysis: Analysis::new(&initial),
            current: initial.clone(),
            initial,
            done: Vec::new(),
            undone: Vec::new(),
            changed: Vec::new(),
            live: config.live_analysis,
            exam: config.exam.then_some(Duration::ZERO),
            theme: config.theme,
            layout: MatrixLayout::Auto,
            hint_level: 0,
            prompt: Prompt::Command,
            editor: LineEditor::new(),
            command_history: CommandHistory::new(),
            matrix_history: CommandHistory::new(),
            feedback: None,
            overlay: Overlay::None,
            quit: false,
        };
        app.feedback = Some(if app.exam.is_some() {
            app.info("Prüfungsmodus: keine Markierungen, Tipps oder Analyse. Viel Erfolg!")
        } else {
            app.info("Willkommen! Gib Zeilenoperationen ein, z. B. »a -2 1 2« – ? zeigt die Hilfe.")
        });
        app
    }

    fn info(&self, text: &str) -> Feedback {
        Feedback {
            tone: Tone::Info,
            text: text.to_owned(),
        }
    }

    // ----- read access -------------------------------------------------

    /// The current matrix `Aₖ`.
    #[must_use]
    pub fn matrix(&self) -> &Matrix {
        &self.current
    }

    /// The initial matrix `A₀`.
    #[must_use]
    pub fn initial(&self) -> &Matrix {
        &self.initial
    }

    /// Structural analysis of the current matrix (always computed).
    #[must_use]
    pub fn analysis(&self) -> &Analysis {
        &self.analysis
    }

    /// The index `k` of the current matrix `Aₖ`.
    #[must_use]
    pub fn step(&self) -> usize {
        self.done.len()
    }

    /// The name of the current matrix, e.g. `A₃`.
    #[must_use]
    pub fn label(&self) -> String {
        matrix_label(self.step())
    }

    /// All steps for the Rechenprotokoll: first the executed ones, then the
    /// undone ones in the order in which redo would replay them.
    #[must_use]
    pub fn protocol(&self) -> Vec<ProtocolEntry<'_>> {
        let done = self.done.iter().enumerate().map(|(i, op)| ProtocolEntry {
            from: i,
            to: i + 1,
            op,
            undone: false,
        });
        let base = self.done.len();
        let undone = self
            .undone
            .iter()
            .rev()
            .enumerate()
            .map(move |(i, op)| ProtocolEntry {
                from: base + i,
                to: base + i + 1,
                op,
                undone: true,
            });
        done.chain(undone).collect()
    }

    /// Whether there is a step to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    /// Whether there is a step to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Zero-based rows changed by the most recent step, undo or redo.
    #[must_use]
    pub fn changed_rows(&self) -> &[usize] {
        &self.changed
    }

    /// Whether exam mode is active.
    #[must_use]
    pub fn is_exam(&self) -> bool {
        self.exam.is_some()
    }

    /// Time spent in exam mode, if active.
    #[must_use]
    pub fn exam_elapsed(&self) -> Option<Duration> {
        self.exam
    }

    /// Whether the live analysis is switched on by the user.
    #[must_use]
    pub fn live_analysis(&self) -> bool {
        self.live
    }

    /// Whether analysis information (highlights, rank, status) may be shown.
    #[must_use]
    pub fn show_analysis(&self) -> bool {
        self.live && self.exam.is_none()
    }

    /// Whether the rows changed by the last step are highlighted.
    #[must_use]
    pub fn show_changes(&self) -> bool {
        self.exam.is_none()
    }

    /// The current theme.
    #[must_use]
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// The current matrix layout.
    #[must_use]
    pub fn layout(&self) -> MatrixLayout {
        self.layout
    }

    /// The message to show, if any.
    #[must_use]
    pub fn feedback(&self) -> Option<&Feedback> {
        self.feedback.as_ref()
    }

    /// The open overlay.
    #[must_use]
    pub fn overlay(&self) -> &Overlay {
        &self.overlay
    }

    /// What the command bar asks for.
    #[must_use]
    pub fn prompt(&self) -> Prompt {
        self.prompt
    }

    /// The command-bar editor.
    #[must_use]
    pub fn editor(&self) -> &LineEditor {
        &self.editor
    }

    /// The seed that regenerates the current matrix, if it was generated.
    #[must_use]
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// The exercise type used for new matrices.
    #[must_use]
    pub fn preset(&self) -> Preset {
        self.preset
    }

    /// The hint level shown last for the current matrix (0 = none).
    #[must_use]
    pub fn hint_level(&self) -> u8 {
        self.hint_level
    }

    /// Whether the program should exit.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    // ----- time ---------------------------------------------------------

    /// Advances the exam clock by `dt` (no effect outside exam mode).
    pub fn tick(&mut self, dt: Duration) {
        if let Some(elapsed) = &mut self.exam {
            *elapsed += dt;
        }
    }

    // ----- input handling ----------------------------------------------

    /// Processes one abstract input event.
    pub fn handle(&mut self, input: Input) {
        if input == Input::Quit {
            self.quit = true;
            return;
        }
        if self.overlay != Overlay::None {
            self.handle_overlay(input);
            return;
        }
        match input {
            Input::Char('?') if self.editor.is_empty() && self.prompt == Prompt::Command => {
                self.execute(Command::Help);
            }
            Input::Char(c) => {
                self.editor.insert(c);
                self.history_mut().stop_browsing();
            }
            Input::Backspace => self.edit(LineEditor::backspace),
            Input::Delete => self.edit(LineEditor::delete),
            Input::DeleteWord => self.edit(LineEditor::delete_word_back),
            Input::ClearLine => self.edit(LineEditor::clear),
            Input::Left => self.editor.left(),
            Input::Right => self.editor.right(),
            Input::Home => self.editor.home(),
            Input::End => self.editor.end(),
            Input::Up => self.browse_history(true),
            Input::Down => self.browse_history(false),
            Input::PageUp | Input::PageDown => {}
            Input::Enter => self.submit(),
            Input::Escape => self.escape(),
            Input::Undo => self.execute(Command::Undo),
            Input::Redo => self.execute(Command::Redo),
            Input::Reset => self.execute(Command::Reset),
            Input::New => self.execute(Command::New(NewSpec::default())),
            Input::ToggleTheme => self.execute(Command::ToggleTheme),
            Input::ToggleAnalysis => self.execute(Command::ToggleAnalysis),
            Input::ToggleExam => self.execute(Command::ToggleExam),
            Input::CycleView => self.execute(Command::CycleView),
            Input::Help => self.execute(Command::Help),
            Input::Hint => self.execute(Command::Hint),
            Input::Check => self.execute(Command::Check(None)),
            Input::Quit => {}
        }
    }

    fn edit(&mut self, f: fn(&mut LineEditor)) {
        f(&mut self.editor);
        self.history_mut().stop_browsing();
    }

    fn history_mut(&mut self) -> &mut CommandHistory {
        match self.prompt {
            Prompt::Command => &mut self.command_history,
            Prompt::NewMatrix | Prompt::CheckMatrix => &mut self.matrix_history,
        }
    }

    fn browse_history(&mut self, older: bool) {
        let current = self.editor.text().to_owned();
        let history = self.history_mut();
        let line = if older {
            history.older(&current).map(str::to_owned)
        } else {
            history.newer().map(str::to_owned)
        };
        if let Some(line) = line {
            self.editor.set(&line);
        }
    }

    fn escape(&mut self) {
        if self.prompt != Prompt::Command {
            self.prompt = Prompt::Command;
            self.editor.clear();
            self.feedback = Some(self.info("Eingabe abgebrochen."));
        } else if !self.editor.is_empty() {
            self.editor.clear();
        } else {
            self.feedback = None;
        }
    }

    fn handle_overlay(&mut self, input: Input) {
        if self.overlay == Overlay::ConfirmSolution {
            match input {
                Input::Char('j' | 'J' | 'y' | 'Y') | Input::Enter => self.show_solution(),
                Input::Char('n' | 'N') | Input::Escape => {
                    self.overlay = Overlay::None;
                    self.feedback = Some(self.info("Gut – versuch es erst einmal selbst."));
                }
                _ => {}
            }
            return;
        }
        // Scrolling is presentation state and handled by the view.
        let close = matches!(
            input,
            Input::Escape | Input::Enter | Input::Char('q' | 'Q') | Input::Help | Input::Char('?')
        );
        if close {
            self.overlay = Overlay::None;
        }
    }

    fn submit(&mut self) {
        let line = self.editor.take();
        match self.prompt {
            Prompt::Command => {
                self.command_history.push(&line);
                match parse_command(&line) {
                    Ok(command) => self.execute(command),
                    Err(e) => {
                        self.feedback = Some(Feedback {
                            tone: Tone::Error,
                            text: e.to_string(),
                        })
                    }
                }
            }
            Prompt::NewMatrix | Prompt::CheckMatrix => match parse_matrix(&line) {
                Ok(m) => {
                    self.matrix_history.push(&line);
                    let prompt = std::mem::take(&mut self.prompt);
                    if prompt == Prompt::NewMatrix {
                        self.set_matrix(m);
                    } else {
                        self.run_check(&m);
                    }
                }
                Err(e) => {
                    // Keep the prompt open with the text so it can be corrected.
                    self.editor.set(&line);
                    self.feedback = Some(Feedback {
                        tone: Tone::Error,
                        text: e.to_string(),
                    });
                }
            },
        }
    }

    // ----- commands -------------------------------------------------------

    /// Executes a parsed command.
    pub fn execute(&mut self, command: Command) {
        match command {
            Command::Operation(op) => self.apply_operation(op),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
            Command::Reset => self.reset(),
            Command::New(spec) => self.new_matrix(&spec),
            Command::SetMatrix(Some(m)) => self.set_matrix(m),
            Command::SetMatrix(None) => {
                self.prompt = Prompt::NewMatrix;
                self.editor.clear();
                self.feedback = Some(
                    self.info("Eigene Matrix eingeben – Zeilen mit ; trennen, z. B. 1 2 3; 4 5 6"),
                );
            }
            Command::Check(matrix) => self.start_check(matrix),
            Command::Hint => self.show_hint(),
            Command::Solution => {
                if self.refuse_in_exam("Die Musterlösung ist im Prüfungsmodus gesperrt.") {
                    return;
                }
                self.overlay = Overlay::ConfirmSolution;
            }
            Command::ToggleAnalysis => {
                if self.refuse_in_exam("Im Prüfungsmodus ist die Live-Analyse ausgeschaltet.") {
                    return;
                }
                self.live = !self.live;
                let text = if self.live {
                    "Live-Analyse eingeschaltet."
                } else {
                    "Live-Analyse ausgeschaltet."
                };
                self.feedback = Some(self.info(text));
            }
            Command::ToggleExam => self.toggle_exam(),
            Command::ToggleTheme => {
                self.theme = self.theme.toggled();
                self.feedback = Some(self.info(&format!("Farbschema: {}.", self.theme.label())));
            }
            Command::CycleView => {
                self.layout = self.layout.cycled();
                self.feedback = Some(self.info(&format!("Ansicht: {}.", self.layout.label())));
            }
            Command::Help => self.overlay = Overlay::Help,
            Command::Quit => self.quit = true,
        }
    }

    fn refuse_in_exam(&mut self, message: &str) -> bool {
        if self.exam.is_some() {
            self.feedback = Some(Feedback {
                tone: Tone::Error,
                text: message.to_owned(),
            });
            true
        } else {
            false
        }
    }

    fn matrix_changed(&mut self) {
        self.analysis = Analysis::new(&self.current);
        self.hint_level = 0;
    }

    /// Recomputes the current matrix from `A₀` and the executed steps.
    fn replay(&mut self) {
        let mut m = self.initial.clone();
        for op in &self.done {
            let result = op.apply(&mut m);
            debug_assert!(result.is_ok(), "history contains an invalid step");
        }
        self.current = m;
        self.matrix_changed();
    }

    fn apply_operation(&mut self, op: RowOperation) {
        if let Err(e) = op.apply(&mut self.current) {
            self.feedback = Some(Feedback {
                tone: Tone::Error,
                text: e.to_string(),
            });
            return;
        }
        self.changed = op.touched_rows();
        self.done.push(op);
        self.undone.clear();
        self.matrix_changed();
        self.feedback = None;
    }

    fn undo(&mut self) {
        let Some(op) = self.done.pop() else {
            self.feedback =
                Some(self.info("Es gibt nichts rückgängig zu machen – du bist bei A₀."));
            return;
        };
        self.changed = op.touched_rows();
        let text = format!("Rückgängig: {}", op.describe());
        self.undone.push(op);
        self.replay();
        self.feedback = Some(self.info(&text));
    }

    fn redo(&mut self) {
        let Some(op) = self.undone.pop() else {
            self.feedback = Some(self.info("Es gibt nichts zu wiederholen."));
            return;
        };
        self.changed = op.touched_rows();
        let text = format!("Wiederholt: {}", op.describe());
        self.done.push(op);
        self.replay();
        self.feedback = Some(self.info(&text));
    }

    fn reset(&mut self) {
        if self.done.is_empty() {
            self.feedback = Some(self.info("Du bist schon bei A₀."));
            return;
        }
        while let Some(op) = self.done.pop() {
            self.undone.push(op);
        }
        self.changed.clear();
        self.replay();
        self.feedback =
            Some(self.info("Zurück bei A₀ – mit Wiederholen (Ctrl+Y) kommst du zurück."));
    }

    fn install(&mut self, matrix: Matrix, seed: Option<u64>) {
        self.initial = matrix.clone();
        self.current = matrix;
        self.seed = seed;
        self.done.clear();
        self.undone.clear();
        self.changed.clear();
        if self.exam.is_some() {
            self.exam = Some(Duration::ZERO);
        }
        self.matrix_changed();
    }

    fn set_matrix(&mut self, matrix: Matrix) {
        let text = format!(
            "Eigene Matrix übernommen ({}×{}).",
            matrix.rows(),
            matrix.cols()
        );
        self.install(matrix, None);
        self.feedback = Some(self.info(&text));
    }

    fn new_matrix(&mut self, spec: &NewSpec) {
        if let Some((rows, cols)) = spec.size {
            self.rows = rows;
            self.cols = cols;
        }
        if let Some(preset) = spec.preset {
            self.preset = preset;
        }
        let seed = match spec.seed {
            Some(s) => {
                self.rng = Rng::new(s);
                s
            }
            None => self.rng.next_u64(),
        };
        let (rows, cols) = self.preset.fit_dims(self.rows, self.cols);
        match generate(&mut Rng::new(seed), rows, cols, self.preset) {
            Ok(m) => {
                let (r, c) = (m.rows(), m.cols());
                self.install(m, Some(seed));
                let mut text = format!(
                    "Neue Aufgabe: {r}×{c} · {} · Seed {seed}",
                    self.preset.label()
                );
                if (r, c) != (self.rows, self.cols) {
                    text.push_str(&format!(
                        " (Größe an die Aufgabenart angepasst: {}×{})",
                        r, c
                    ));
                }
                self.feedback = Some(self.info(&text));
            }
            Err(e) => {
                self.feedback = Some(Feedback {
                    tone: Tone::Error,
                    text: e.to_string(),
                })
            }
        }
    }

    fn start_check(&mut self, matrix: Option<Matrix>) {
        if self.refuse_in_exam("Die Kontrolle ist im Prüfungsmodus gesperrt.") {
            return;
        }
        match matrix {
            Some(m) => self.run_check(&m),
            None => {
                self.prompt = Prompt::CheckMatrix;
                self.editor.clear();
                self.feedback = Some(self.info(&format!(
                    "Deine Matrix eingeben (Zeilen mit ; trennen) – sie wird mit {} und der Treppennormalform verglichen.",
                    self.label()
                )));
            }
        }
    }

    fn run_check(&mut self, entered: &Matrix) {
        let report = check(entered, &self.current);
        self.overlay = Overlay::Check {
            at_step: self.step(),
            report: Box::new(report),
        };
        self.feedback = None;
    }

    fn show_hint(&mut self) {
        if self.refuse_in_exam("Tipps sind im Prüfungsmodus gesperrt.") {
            return;
        }
        self.hint_level = (self.hint_level + 1).min(MAX_LEVEL);
        let h = hint(self.hint_level, &self.current);
        self.feedback = Some(Feedback {
            tone: Tone::Hint,
            text: format!("Tipp {}/{MAX_LEVEL}: {}", h.level, h.text),
        });
    }

    fn show_solution(&mut self) {
        self.overlay = Overlay::Solution {
            from_step: self.step(),
            solution: Box::new(solve(&self.current)),
        };
    }

    fn toggle_exam(&mut self) {
        match self.exam.take() {
            None => {
                self.exam = Some(Duration::ZERO);
                self.hint_level = 0;
                self.feedback = Some(self.info(
                    "Prüfungsmodus: keine Markierungen, Tipps, Kontrolle oder Live-Analyse. Der Timer läuft.",
                ));
            }
            Some(elapsed) => {
                let solved = self.current == rref(&self.current);
                let steps = self.done.len();
                let verdict = if solved {
                    "Treppennormalform erreicht ✓"
                } else {
                    "Treppennormalform nicht erreicht"
                };
                self.feedback = Some(Feedback {
                    tone: if solved { Tone::Success } else { Tone::Info },
                    text: format!(
                        "Prüfung beendet nach {} · {steps} Schritte · {verdict}",
                        format_clock(elapsed)
                    ),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_command;
    use crate::rational::int;

    fn example_app() -> App {
        App::new(AppConfig {
            matrix: Some(Matrix::course_example()),
            ..AppConfig::default()
        })
    }

    fn run(app: &mut App, line: &str) {
        app.execute(parse_command(line).unwrap());
    }

    fn type_line(app: &mut App, line: &str) {
        for c in line.chars() {
            app.handle(Input::Char(c));
        }
        app.handle(Input::Enter);
    }

    #[test]
    fn starts_with_a_generated_matrix_of_the_requested_size() {
        let app = App::new(AppConfig {
            seed: 7,
            rows: 2,
            cols: 5,
            ..AppConfig::default()
        });
        assert_eq!((app.matrix().rows(), app.matrix().cols()), (2, 5));
        assert_eq!(app.seed(), Some(7));
        assert_eq!(app.step(), 0);
        assert_eq!(app.label(), "A₀");
        assert_eq!(app.matrix(), app.initial());
    }

    #[test]
    fn same_seed_gives_the_same_session() {
        let mut a = App::new(AppConfig {
            seed: 99,
            ..AppConfig::default()
        });
        let mut b = App::new(AppConfig {
            seed: 99,
            ..AppConfig::default()
        });
        assert_eq!(a.matrix(), b.matrix());
        a.execute(Command::New(NewSpec::default()));
        b.execute(Command::New(NewSpec::default()));
        assert_eq!(a.matrix(), b.matrix());
        assert_eq!(a.seed(), b.seed());
    }

    #[test]
    fn new_with_seed_reproduces_the_startup_matrix() {
        let first = App::new(AppConfig {
            seed: 5,
            ..AppConfig::default()
        });
        let mut other = App::new(AppConfig {
            seed: 123,
            ..AppConfig::default()
        });
        run(&mut other, "new seed=5");
        assert_eq!(other.matrix(), first.matrix());
    }

    #[test]
    fn operations_are_logged_and_numbered() {
        let mut app = example_app();
        run(&mut app, "a -2 1 2");
        run(&mut app, "a -3 1 3");
        assert_eq!(app.step(), 2);
        assert_eq!(app.label(), "A₂");
        assert_eq!(app.changed_rows(), &[2]);
        let log = app.protocol();
        assert_eq!(log.len(), 2);
        assert_eq!((log[1].from, log[1].to), (1, 2));
        assert_eq!(
            log[0].op.describe(),
            "Das (-2)-fache von Zeile 1 wird zu Zeile 2 addiert."
        );
        assert_eq!(app.matrix()[(1, 0)], int(0));
    }

    #[test]
    fn invalid_operations_leave_the_state_untouched() {
        let mut app = example_app();
        let before = app.matrix().clone();
        run(&mut app, "s 1 9");
        assert_eq!(app.step(), 0);
        assert_eq!(app.matrix(), &before);
        let fb = app.feedback().unwrap();
        assert_eq!(fb.tone, Tone::Error);
        assert!(fb.text.contains("Zeile 9 gibt es nicht"));
    }

    #[test]
    fn undo_and_redo_walk_the_history() {
        let mut app = example_app();
        run(&mut app, "a -2 1 2");
        run(&mut app, "s 2 3");
        let after_two = app.matrix().clone();
        run(&mut app, "u");
        assert_eq!(app.step(), 1);
        assert!(app.can_redo());
        run(&mut app, "u");
        assert_eq!(app.matrix(), app.initial());
        assert!(!app.can_undo());
        run(&mut app, "u");
        assert_eq!(app.feedback().unwrap().tone, Tone::Info);
        run(&mut app, "r");
        run(&mut app, "r");
        assert_eq!(app.matrix(), &after_two);
        run(&mut app, "r");
        assert!(
            app.feedback()
                .unwrap()
                .text
                .contains("nichts zu wiederholen")
        );
    }

    #[test]
    fn protocol_shows_undone_steps_after_the_executed_ones() {
        let mut app = example_app();
        run(&mut app, "s 1 2");
        run(&mut app, "s 2 3");
        run(&mut app, "u");
        run(&mut app, "u");
        let log = app.protocol();
        assert_eq!(log.len(), 2);
        assert!(log.iter().all(|e| e.undone));
        assert_eq!(log[0].op.describe(), "Zeile 1 wird mit Zeile 2 vertauscht.");
        assert_eq!((log[1].from, log[1].to), (1, 2));
    }

    #[test]
    fn a_new_operation_discards_the_redo_stack() {
        let mut app = example_app();
        run(&mut app, "s 1 2");
        run(&mut app, "u");
        run(&mut app, "s 1 3");
        assert!(!app.can_redo());
        assert_eq!(app.protocol().len(), 1);
    }

    #[test]
    fn reset_returns_to_a0_and_is_reversible() {
        let mut app = example_app();
        run(&mut app, "a -2 1 2");
        run(&mut app, "m 3 1/2");
        let work = app.matrix().clone();
        run(&mut app, "reset");
        assert_eq!(app.matrix(), app.initial());
        assert_eq!(app.step(), 0);
        run(&mut app, "r");
        run(&mut app, "r");
        assert_eq!(app.matrix(), &work);
        run(&mut app, "reset");
        run(&mut app, "reset");
        assert!(app.feedback().unwrap().text.contains("schon bei A₀"));
    }

    #[test]
    fn solving_the_example_by_commands_reaches_the_expected_result() {
        let mut app = example_app();
        for line in [
            "a -2 1 2", "a -3 1 3", "m 2 1/3", "a 2 2 1", "a -7 2 3", "m 3 3/10", "a 1 3 2",
            "a -1 3 1",
        ] {
            run(&mut app, line);
        }
        // Not the solver's path, but any correct path ends in the same matrix.
        for op in solve(app.matrix()).steps {
            app.execute(Command::Operation(op.op));
        }
        let expected = parse_matrix("[[1,0,0,15/7],[0,1,0,-4/7],[0,0,1,-10/7]]").unwrap();
        assert_eq!(app.matrix(), &expected);
        assert!(app.analysis().is_rref());
        assert_eq!(app.analysis().rank(), 3);
    }

    #[test]
    fn new_applies_size_and_preset_and_reports_adjustments() {
        let mut app = example_app();
        run(&mut app, "new 3x4 hoch seed=3");
        assert_eq!(app.preset(), Preset::MoreRows);
        assert!(app.matrix().rows() > app.matrix().cols());
        assert!(app.feedback().unwrap().text.contains("angepasst"));
        assert_eq!(app.step(), 0);
        assert_eq!(app.seed(), Some(3));
    }

    #[test]
    fn set_matrix_installs_a_user_matrix() {
        let mut app = example_app();
        run(&mut app, "s 1 2");
        run(&mut app, "set [[1,2],[3,4]]");
        assert_eq!((app.matrix().rows(), app.matrix().cols()), (2, 2));
        assert_eq!(app.step(), 0);
        assert_eq!(app.seed(), None);
        assert!(!app.can_undo() && !app.can_redo());
    }

    #[test]
    fn matrix_prompt_keeps_the_text_after_an_error() {
        let mut app = example_app();
        type_line(&mut app, "set");
        assert_eq!(app.prompt(), Prompt::NewMatrix);
        type_line(&mut app, "1 2; 3");
        assert_eq!(app.prompt(), Prompt::NewMatrix);
        assert_eq!(app.editor().text(), "1 2; 3");
        assert_eq!(app.feedback().unwrap().tone, Tone::Error);
        app.handle(Input::Char(' '));
        app.handle(Input::Char('4'));
        app.handle(Input::Enter);
        assert_eq!(app.prompt(), Prompt::Command);
        assert_eq!(app.matrix().rows(), 2);
    }

    #[test]
    fn escape_cancels_a_matrix_prompt() {
        let mut app = example_app();
        type_line(&mut app, "check");
        assert_eq!(app.prompt(), Prompt::CheckMatrix);
        app.handle(Input::Char('1'));
        app.handle(Input::Escape);
        assert_eq!(app.prompt(), Prompt::Command);
        assert!(app.editor().is_empty());
    }

    #[test]
    fn check_compares_with_the_current_matrix_and_the_normal_form() {
        let mut app = example_app();
        run(&mut app, "a -2 1 2");
        let Command::Check(Some(entered)) =
            parse_command("check [[1,-2,3,-1],[0,3,-4,4],[3,1,2,3]]").unwrap()
        else {
            panic!("expected check");
        };
        app.execute(Command::Check(Some(entered)));
        let Overlay::Check { at_step, report } = app.overlay() else {
            panic!("expected overlay")
        };
        assert_eq!(*at_step, 1);
        assert!(report.matches_current);
        app.handle(Input::Escape);
        assert_eq!(*app.overlay(), Overlay::None);
        type_line(&mut app, "check");
        type_line(&mut app, "1 0 0 15/7; 0 1 0 -4/7; 0 0 1 -10/7");
        let Overlay::Check { report, .. } = app.overlay() else {
            panic!("expected overlay")
        };
        assert!(report.matches_rref && !report.matches_current);
    }

    #[test]
    fn hints_escalate_per_matrix_and_reset_after_a_step() {
        let mut app = example_app();
        run(&mut app, "tipp");
        assert!(app.feedback().unwrap().text.starts_with("Tipp 1/3"));
        run(&mut app, "tipp");
        assert!(app.feedback().unwrap().text.starts_with("Tipp 2/3"));
        run(&mut app, "tipp");
        assert!(app.feedback().unwrap().text.contains("a -2 1 2"));
        run(&mut app, "tipp");
        assert!(app.feedback().unwrap().text.starts_with("Tipp 3/3"));
        run(&mut app, "a -2 1 2");
        assert_eq!(app.hint_level(), 0);
        run(&mut app, "tipp");
        assert!(app.feedback().unwrap().text.starts_with("Tipp 1/3"));
    }

    #[test]
    fn hints_are_never_shown_unprompted() {
        let mut app = example_app();
        run(&mut app, "s 1 2");
        run(&mut app, "m 1 1/2");
        assert_eq!(app.hint_level(), 0);
        assert!(app.feedback().is_none());
    }

    #[test]
    fn the_solution_needs_confirmation() {
        let mut app = example_app();
        run(&mut app, "lösung");
        assert_eq!(*app.overlay(), Overlay::ConfirmSolution);
        app.handle(Input::Char('n'));
        assert_eq!(*app.overlay(), Overlay::None);
        run(&mut app, "lösung");
        app.handle(Input::Char('j'));
        let Overlay::Solution {
            solution,
            from_step,
            ..
        } = app.overlay()
        else {
            panic!("expected solution")
        };
        assert_eq!(*from_step, 0);
        assert_eq!(solution.result, rref(&Matrix::course_example()));
        app.handle(Input::Escape);
        assert_eq!(*app.overlay(), Overlay::None);
    }

    #[test]
    fn exam_mode_locks_assistance_and_runs_a_timer() {
        let mut app = example_app();
        run(&mut app, "exam");
        assert!(app.is_exam());
        assert!(!app.show_analysis() && !app.show_changes());
        for cmd in ["tipp", "lösung", "check", "analyse"] {
            run(&mut app, cmd);
            assert_eq!(app.feedback().unwrap().tone, Tone::Error, "{cmd}");
            assert_eq!(*app.overlay(), Overlay::None, "{cmd}");
        }
        app.tick(Duration::from_secs(83));
        assert_eq!(format_clock(app.exam_elapsed().unwrap()), "01:23");
        run(&mut app, "s 1 2");
        assert_eq!(app.step(), 1);
        run(&mut app, "exam");
        assert!(!app.is_exam());
        assert!(
            app.feedback()
                .unwrap()
                .text
                .contains("Prüfung beendet nach 01:23")
        );
        assert!(app.feedback().unwrap().text.contains("nicht erreicht"));
        assert!(app.show_analysis());
    }

    #[test]
    fn exam_verdict_recognises_a_solved_matrix() {
        let mut app = example_app();
        run(&mut app, "exam");
        for step in solve(app.matrix()).steps {
            app.execute(Command::Operation(step.op));
        }
        run(&mut app, "exam");
        let fb = app.feedback().unwrap();
        assert_eq!(fb.tone, Tone::Success);
        assert!(fb.text.contains("Treppennormalform erreicht ✓"));
    }

    #[test]
    fn the_clock_only_runs_in_exam_mode() {
        let mut app = example_app();
        app.tick(Duration::from_secs(10));
        assert_eq!(app.exam_elapsed(), None);
        run(&mut app, "exam");
        assert_eq!(app.exam_elapsed(), Some(Duration::ZERO));
    }

    #[test]
    fn toggles_flip_state() {
        let mut app = example_app();
        assert_eq!(app.theme(), Theme::Dark);
        app.handle(Input::ToggleTheme);
        assert_eq!(app.theme(), Theme::Light);
        assert!(app.live_analysis());
        app.handle(Input::ToggleAnalysis);
        assert!(!app.show_analysis());
        assert_eq!(app.layout(), MatrixLayout::Auto);
        app.handle(Input::CycleView);
        assert_eq!(app.layout(), MatrixLayout::Stacked);
        app.handle(Input::CycleView);
        app.handle(Input::CycleView);
        assert_eq!(app.layout(), MatrixLayout::Auto);
    }

    #[test]
    fn question_mark_opens_help_only_on_an_empty_line() {
        let mut app = example_app();
        app.handle(Input::Char('?'));
        assert_eq!(*app.overlay(), Overlay::Help);
        app.handle(Input::Char('?'));
        assert_eq!(*app.overlay(), Overlay::None);
        app.handle(Input::Char('x'));
        app.handle(Input::Char('?'));
        assert_eq!(app.editor().text(), "x?");
        assert_eq!(*app.overlay(), Overlay::None);
    }

    #[test]
    fn command_history_recalls_previous_lines() {
        let mut app = example_app();
        type_line(&mut app, "s 1 2");
        type_line(&mut app, "m 1 2");
        app.handle(Input::Up);
        assert_eq!(app.editor().text(), "m 1 2");
        app.handle(Input::Up);
        assert_eq!(app.editor().text(), "s 1 2");
        app.handle(Input::Down);
        app.handle(Input::Down);
        assert_eq!(app.editor().text(), "");
    }

    #[test]
    fn bad_input_gives_friendly_errors() {
        let mut app = example_app();
        for line in ["blah", "m 1", "m 1 0", "a 1/0 1 2", "s 1 1", "m x 2"] {
            type_line(&mut app, line);
            let fb = app.feedback().unwrap();
            assert_eq!(fb.tone, Tone::Error, "{line}");
            assert!(!fb.text.is_empty());
        }
        assert_eq!(app.step(), 0);
    }

    #[test]
    fn quit_works_from_anywhere() {
        let mut app = example_app();
        run(&mut app, "lösung");
        app.handle(Input::Quit);
        assert!(app.should_quit());
        let mut app = example_app();
        type_line(&mut app, "q");
        assert!(app.should_quit());
    }

    #[test]
    fn formats_the_exam_clock() {
        assert_eq!(format_clock(Duration::from_secs(0)), "00:00");
        assert_eq!(format_clock(Duration::from_secs(61)), "01:01");
        assert_eq!(format_clock(Duration::from_secs(3_725)), "62:05");
    }
}
