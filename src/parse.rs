//! Parsing of numbers, matrices and commands.
//!
//! All parsers are total: any input yields either a value or a
//! [`ParseError`] with a friendly German message, never a panic.
//!
//! # Numbers
//!
//! Integers (`3`, `-7`), fractions (`-3/4`) and decimals (`0.25`, `0,25`,
//! `.5`) are accepted and converted **exactly**: `0.25` becomes `1/4`.
//! The true minus sign `−` is accepted as well. Exponent notation is not.
//!
//! # Matrices
//!
//! * nested: `[[1,-2,3],[2,-1,2]]`
//! * flat: `1 -2 3; 2 -1 2` (rows separated by `;`, `|` or a newline,
//!   entries by whitespace, optionally wrapped in one pair of brackets)
//!
//! # Commands
//!
//! See [`parse_command`] for the full command language.

use num_bigint::BigInt;
use thiserror::Error;

use crate::generator::Preset;
use crate::matrix::{MAX_DIM, Matrix, MatrixError};
use crate::ops::{OpError, RowOperation};
use crate::rational::Q;

/// Longest accepted number literal, in characters.
const MAX_NUMBER_LEN: usize = 40;

/// Why a number literal was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NumberError {
    /// Nothing was entered.
    #[error("Bitte gib eine Zahl ein.")]
    Empty,
    /// The text is not a number in a supported notation.
    #[error("»{0}« ist keine gültige Zahl. Erlaubt sind z. B. 3, -3/4 oder 0.25.")]
    Invalid(String),
    /// A fraction with denominator zero.
    #[error("Der Nenner darf nicht 0 sein.")]
    ZeroDenominator,
    /// The literal has more than 40 characters.
    #[error("Die Zahl »{0}…« ist zu lang (höchstens {MAX_NUMBER_LEN} Zeichen).")]
    TooLong(String),
}

/// Why an input line was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// The command line was empty.
    #[error("Bitte gib einen Befehl ein – ? zeigt die Hilfe.")]
    EmptyInput,
    /// The first word is not a command.
    #[error("Unbekannter Befehl »{0}« – ? zeigt alle Befehle.")]
    UnknownCommand(String),
    /// The wrong number of arguments for a command.
    #[error("Falsche Eingabe. Aufruf: {usage}")]
    Usage {
        /// One-line usage text.
        usage: &'static str,
    },
    /// Arguments were given to a command that takes none.
    #[error("Der Befehl »{0}« erwartet keine Argumente.")]
    UnexpectedArguments(String),
    /// A row index that is not a positive integer.
    #[error("»{0}« ist keine gültige Zeilennummer – erlaubt sind ganze Zahlen ab 1.")]
    InvalidIndex(String),
    /// A malformed number literal.
    #[error(transparent)]
    Number(#[from] NumberError),
    /// A malformed entry inside a matrix.
    #[error("Eintrag in Zeile {row}, Spalte {col}: {source}")]
    Entry {
        /// One-based row of the entry.
        row: usize,
        /// One-based column of the entry.
        col: usize,
        /// What is wrong with it.
        #[source]
        source: NumberError,
    },
    /// The matrix text could not be understood.
    #[error("Die Matrix konnte nicht gelesen werden: {0}")]
    Malformed(&'static str),
    /// A well-formed matrix of unsupported shape.
    #[error(transparent)]
    Matrix(#[from] MatrixError),
    /// An operation with illegal operands (zero factor, identical rows).
    #[error(transparent)]
    Operation(#[from] OpError),
    /// A size such as `3x4` that could not be read.
    #[error("»{0}« ist keine gültige Größe – verwende z. B. 3x4.")]
    InvalidSize(String),
    /// A seed that is not a non-negative integer.
    #[error("»{0}« ist kein gültiger Seed – verwende z. B. seed=42.")]
    InvalidSeed(String),
    /// An argument of `new` that is neither size, seed nor preset.
    #[error(
        "»{0}« ist weder eine Größe (z. B. 3x4), ein Seed (seed=42) noch eine Aufgabenart \
         (allgemein, a11, abhängig, breit, hoch)."
    )]
    InvalidNewArgument(String),
}

/// Arguments of the `new` command; every part is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewSpec {
    /// Requested size `(rows, cols)`.
    pub size: Option<(usize, usize)>,
    /// Requested exercise type.
    pub preset: Option<Preset>,
    /// Requested seed for reproducible matrices.
    pub seed: Option<u64>,
}

/// A parsed command-bar input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// An elementary row operation.
    Operation(RowOperation),
    /// Undo the last step.
    Undo,
    /// Redo the last undone step.
    Redo,
    /// Return to A₀.
    Reset,
    /// Generate a new matrix.
    New(NewSpec),
    /// Use a user-supplied matrix; `None` opens the input prompt.
    SetMatrix(Option<Matrix>),
    /// Compare a hand-computed matrix; `None` opens the input prompt.
    Check(Option<Matrix>),
    /// Show the next hint level.
    Hint,
    /// Show the reference solution (after confirmation).
    Solution,
    /// Toggle the live analysis.
    ToggleAnalysis,
    /// Toggle exam mode.
    ToggleExam,
    /// Toggle the dark/light theme.
    ToggleTheme,
    /// Cycle the matrix layout (auto, stacked, inline).
    CycleView,
    /// Show the help overlay.
    Help,
    /// Quit the program.
    Quit,
}

/// Parses an exact rational number.
///
/// ```
/// use gauss_trainer::parse::parse_number;
/// use gauss_trainer::rational::{int, ratio};
///
/// assert_eq!(parse_number("-3/4").unwrap(), ratio(-3, 4).unwrap());
/// assert_eq!(parse_number("0.25").unwrap(), ratio(1, 4).unwrap());
/// assert_eq!(parse_number("0,5").unwrap(), ratio(1, 2).unwrap());
/// assert_eq!(parse_number("7").unwrap(), int(7));
/// assert!(parse_number("1/0").is_err());
/// ```
///
/// # Errors
///
/// Returns a [`NumberError`] for empty, malformed or over-long input and for
/// zero denominators.
pub fn parse_number(input: &str) -> Result<Q, NumberError> {
    let text = input.trim().replace('−', "-");
    if text.is_empty() {
        return Err(NumberError::Empty);
    }
    if text.chars().count() > MAX_NUMBER_LEN {
        return Err(NumberError::TooLong(text.chars().take(10).collect()));
    }
    let invalid = || NumberError::Invalid(input.trim().to_owned());

    let (negative, unsigned) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text.as_str()),
    };

    let (numer, denom) = if let Some((n, d)) = unsigned.split_once('/') {
        if !is_digits(n) || !is_digits(d) {
            return Err(invalid());
        }
        let numer: BigInt = n.parse().map_err(|_| invalid())?;
        let denom: BigInt = d.parse().map_err(|_| invalid())?;
        (numer, denom)
    } else {
        let (int_part, frac_part) = match unsigned.split_once(['.', ',']) {
            Some((i, f)) => (i, f),
            None => (unsigned, ""),
        };
        let has_separator = unsigned.len() != int_part.len();
        let digits_ok = (int_part.is_empty() || is_digits(int_part))
            && (frac_part.is_empty() || is_digits(frac_part))
            && !(int_part.is_empty() && frac_part.is_empty());
        // "5." and "." are not numbers; ".5" is.
        if !digits_ok || (has_separator && frac_part.is_empty()) {
            return Err(invalid());
        }
        let combined = format!("{int_part}{frac_part}");
        let numer: BigInt = combined.parse().map_err(|_| invalid())?;
        let scale = u32::try_from(frac_part.len()).map_err(|_| invalid())?;
        (numer, BigInt::from(10u8).pow(scale))
    };

    if num_traits::Zero::is_zero(&denom) {
        return Err(NumberError::ZeroDenominator);
    }
    let value = Q::new(numer, denom);
    Ok(if negative { -value } else { value })
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Parses a matrix from nested (`[[1,2],[3,4]]`) or flat (`1 2; 3 4`) notation.
///
/// # Errors
///
/// Returns a [`ParseError`] for unbalanced brackets, malformed entries,
/// rows of different length, or sizes outside `1..=6` per dimension.
pub fn parse_matrix(input: &str) -> Result<Matrix, ParseError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(MatrixError::Empty.into());
    }
    let inner = match (text.strip_prefix('['), text.ends_with(']')) {
        (Some(rest), true) => rest[..rest.len() - 1].trim(),
        (None, false) => text,
        _ => {
            return Err(ParseError::Malformed(
                "Die eckigen Klammern passen nicht zusammen.",
            ));
        }
    };

    let row_texts: Vec<Vec<&str>> = if inner.starts_with('[') {
        split_nested_rows(inner)?
    } else {
        inner
            .split([';', '|', '\n'])
            .map(|row| row.split_whitespace().collect::<Vec<_>>())
            .filter(|cells| !cells.is_empty())
            .collect()
    };

    if row_texts.is_empty() {
        return Err(MatrixError::Empty.into());
    }
    if row_texts.len() > MAX_DIM {
        let cols = row_texts.iter().map(Vec::len).max().unwrap_or(0);
        return Err(MatrixError::TooLarge {
            rows: row_texts.len(),
            cols,
        }
        .into());
    }

    let mut rows = Vec::with_capacity(row_texts.len());
    for (i, cells) in row_texts.iter().enumerate() {
        if cells.len() > MAX_DIM {
            return Err(MatrixError::TooLarge {
                rows: row_texts.len(),
                cols: cells.len(),
            }
            .into());
        }
        let row = cells
            .iter()
            .enumerate()
            .map(|(j, cell)| {
                parse_number(cell).map_err(|source| ParseError::Entry {
                    row: i + 1,
                    col: j + 1,
                    source,
                })
            })
            .collect::<Result<Vec<Q>, _>>()?;
        rows.push(row);
    }
    Ok(Matrix::from_rows(rows)?)
}

/// Splits `[1,2],[3,4]` into rows of cell texts.
fn split_nested_rows(inner: &str) -> Result<Vec<Vec<&str>>, ParseError> {
    const BAD: ParseError = ParseError::Malformed("Die eckigen Klammern passen nicht zusammen.");
    let mut rows = Vec::new();
    let mut rest = inner.trim();
    while !rest.is_empty() {
        let body = rest.strip_prefix('[').ok_or(BAD)?;
        let end = body.find(']').ok_or(BAD)?;
        let (cells, tail) = body.split_at(end);
        if cells.contains('[') {
            return Err(BAD);
        }
        rows.push(
            cells
                .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .collect(),
        );
        rest = tail[1..].trim_start_matches(|c: char| c == ',' || c == ';' || c.is_whitespace());
    }
    Ok(rows)
}

fn parse_index(token: &str) -> Result<usize, ParseError> {
    match token.parse::<usize>() {
        Ok(n) if n >= 1 && is_digits(token) => Ok(n - 1),
        _ => Err(ParseError::InvalidIndex(token.to_owned())),
    }
}

fn parse_size(token: &str) -> Result<(usize, usize), ParseError> {
    let bad = || ParseError::InvalidSize(token.to_owned());
    let (r, c) = token.split_once(['x', '×', '*']).ok_or_else(bad)?;
    if !is_digits(r) || !is_digits(c) {
        return Err(bad());
    }
    let rows: usize = r.parse().map_err(|_| bad())?;
    let cols: usize = c.parse().map_err(|_| bad())?;
    if rows == 0 || cols == 0 {
        return Err(MatrixError::Empty.into());
    }
    if rows > MAX_DIM || cols > MAX_DIM {
        return Err(MatrixError::TooLarge { rows, cols }.into());
    }
    Ok((rows, cols))
}

fn parse_new_spec(args: &[&str]) -> Result<NewSpec, ParseError> {
    let mut spec = NewSpec::default();
    for &arg in args {
        let lower = arg.to_lowercase();
        if let Some(value) = lower
            .strip_prefix("seed=")
            .or_else(|| lower.strip_prefix('#'))
        {
            if !is_digits(value) {
                return Err(ParseError::InvalidSeed(value.to_owned()));
            }
            spec.seed = Some(
                value
                    .parse()
                    .map_err(|_| ParseError::InvalidSeed(value.to_owned()))?,
            );
        } else if lower.contains(['x', '×', '*']) && lower.starts_with(|c: char| c.is_ascii_digit())
        {
            spec.size = Some(parse_size(&lower)?);
        } else if let Some(preset) = Preset::from_keyword(&lower) {
            spec.preset = Some(preset);
        } else {
            return Err(ParseError::InvalidNewArgument(arg.to_owned()));
        }
    }
    Ok(spec)
}

const USAGE_MUL: &str = "m <i> <c>  –  Zeile i mit dem Faktor c multiplizieren, z. B. m 1 -1/2";
const USAGE_ADD: &str =
    "a <c> <i> <j>  –  das c-fache von Zeile i zu Zeile j addieren, z. B. a -2 1 2";
const USAGE_SWAP: &str = "s <i> <j>  –  Zeile i mit Zeile j vertauschen, z. B. s 1 2";

/// Parses one line of the command bar.
///
/// | Input | Meaning |
/// |-------|---------|
/// | `m <i> <c>` (`mul`, `mal`) | multiply row *i* by *c* |
/// | `a <c> <i> <j>` (`add`) | add the *c*-fold of row *i* to row *j* |
/// | `s <i> <j>` (`swap`, `tausch`) | swap rows *i* and *j* |
/// | `u`, `undo`, `z` | undo |
/// | `r`, `redo`, `y` | redo |
/// | `reset` | back to A₀ |
/// | `n [MxN] [Art] [seed=K]` (`new`, `neu`) | new matrix |
/// | `set [Matrix]` (`e`, `eingabe`) | enter your own matrix |
/// | `check [Matrix]` (`c`, `prüfen`) | check a hand-computed matrix |
/// | `tipp`, `t`, `hint` | next hint level |
/// | `lösung`, `solution` | reference solution |
/// | `analyse`, `exam`, `theme`, `ansicht` | toggles |
/// | `?`, `h`, `help`, `hilfe` | help |
/// | `q`, `quit`, `exit` | quit |
///
/// Commands are case-insensitive. Row numbers count from 1.
///
/// ```
/// use gauss_trainer::parse::{parse_command, Command};
///
/// let Command::Operation(op) = parse_command("a -2 1 3").unwrap() else { panic!() };
/// assert_eq!(op.describe(), "Das (-2)-fache von Zeile 1 wird zu Zeile 3 addiert.");
/// assert!(parse_command("m 1 0").is_err());
/// ```
///
/// # Errors
///
/// Returns a [`ParseError`] describing what is wrong with the input.
pub fn parse_command(input: &str) -> Result<Command, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }
    let (word, rest) = trimmed
        .split_once(char::is_whitespace)
        .map_or((trimmed, ""), |(w, r)| (w, r.trim()));
    let key = word.to_lowercase();
    let args: Vec<&str> = rest.split_whitespace().collect();

    let no_args = |command: Command| {
        if args.is_empty() {
            Ok(command)
        } else {
            Err(ParseError::UnexpectedArguments(word.to_owned()))
        }
    };
    let optional_matrix = || {
        if rest.is_empty() {
            Ok(None)
        } else {
            parse_matrix(rest).map(Some)
        }
    };

    match key.as_str() {
        "m" | "mul" | "mult" | "mal" | "multipliziere" => {
            let [i, c] = args[..] else {
                return Err(ParseError::Usage { usage: USAGE_MUL });
            };
            Ok(Command::Operation(RowOperation::scale(
                parse_index(i)?,
                parse_number(c)?,
            )?))
        }
        "a" | "add" | "addiere" => {
            let [c, i, j] = args[..] else {
                return Err(ParseError::Usage { usage: USAGE_ADD });
            };
            let op =
                RowOperation::add_multiple(parse_number(c)?, parse_index(i)?, parse_index(j)?)?;
            Ok(Command::Operation(op))
        }
        "s" | "swap" | "tausch" | "tausche" | "vertausche" => {
            let [i, j] = args[..] else {
                return Err(ParseError::Usage { usage: USAGE_SWAP });
            };
            Ok(Command::Operation(RowOperation::swap(
                parse_index(i)?,
                parse_index(j)?,
            )?))
        }
        "u" | "z" | "undo" | "zurück" | "zurueck" => no_args(Command::Undo),
        "r" | "y" | "redo" | "wiederholen" => no_args(Command::Redo),
        "reset" | "zurücksetzen" | "zuruecksetzen" => no_args(Command::Reset),
        "n" | "new" | "neu" => Ok(Command::New(parse_new_spec(&args)?)),
        "e" | "set" | "eingabe" | "matrix" => Ok(Command::SetMatrix(optional_matrix()?)),
        "c" | "check" | "prüfen" | "pruefen" | "kontrolle" => {
            Ok(Command::Check(optional_matrix()?))
        }
        "t" | "tipp" | "hint" | "hinweis" => no_args(Command::Hint),
        "lösung" | "loesung" | "solution" | "musterlösung" => no_args(Command::Solution),
        "analyse" | "analysis" | "live" => no_args(Command::ToggleAnalysis),
        "exam" | "prüfung" | "pruefung" => no_args(Command::ToggleExam),
        "theme" | "thema" => no_args(Command::ToggleTheme),
        "v" | "view" | "ansicht" => no_args(Command::CycleView),
        "h" | "?" | "help" | "hilfe" => no_args(Command::Help),
        "q" | "quit" | "exit" | "ende" | "beenden" => no_args(Command::Quit),
        _ => Err(ParseError::UnknownCommand(word.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::{int, ratio};

    fn num(s: &str) -> Q {
        parse_number(s).unwrap()
    }

    #[test]
    fn parses_integers_fractions_and_decimals() {
        assert_eq!(num("42"), int(42));
        assert_eq!(num("-7"), int(-7));
        assert_eq!(num("+7"), int(7));
        assert_eq!(num("−7"), int(-7));
        assert_eq!(num("-3/4"), ratio(-3, 4).unwrap());
        assert_eq!(num("6/4"), ratio(3, 2).unwrap());
        assert_eq!(num("0.25"), ratio(1, 4).unwrap());
        assert_eq!(num("-0,125"), ratio(-1, 8).unwrap());
        assert_eq!(num(".5"), ratio(1, 2).unwrap());
        assert_eq!(num("  2.50 "), ratio(5, 2).unwrap());
        assert_eq!(num("007"), int(7));
        assert_eq!(num("-0"), int(0));
    }

    #[test]
    fn decimals_are_exact() {
        // 0.1 + 0.2 == 0.3 holds only in exact arithmetic.
        assert_eq!(num("0.1") + num("0.2"), num("0.3"));
        assert_eq!(num("0.333333333333333333333333"), {
            let nine = BigInt::from(10u8).pow(24);
            Q::new(BigInt::from(333_333_333_333_333_333_333_333u128), nine)
        });
    }

    #[test]
    fn rejects_malformed_numbers() {
        for bad in [
            "", "   ", "abc", "1/", "/2", "1/2/3", "3/-4", "--1", "+-1", "1e3", "5.", ".", "1.2.3",
            "0x10", "1 2", "½", "1/2.5", "∞", "NaN", "-", "+",
        ] {
            assert!(parse_number(bad).is_err(), "{bad:?} should be rejected");
        }
        assert_eq!(parse_number("1/0"), Err(NumberError::ZeroDenominator));
        assert_eq!(parse_number(""), Err(NumberError::Empty));
        assert!(matches!(
            parse_number(&"9".repeat(80)),
            Err(NumberError::TooLong(_))
        ));
    }

    #[test]
    fn number_errors_are_friendly() {
        let msg = parse_number("abc").unwrap_err().to_string();
        assert!(msg.contains("»abc« ist keine gültige Zahl"), "{msg}");
    }

    #[test]
    fn parses_nested_matrices() {
        let m = parse_matrix("[[1,-2,3,-1],[2,-1,2,2],[3,1,2,3]]").unwrap();
        assert_eq!((m.rows(), m.cols()), (3, 4));
        assert_eq!(m[(0, 1)], int(-2));
        let spaced = parse_matrix(" [ [1, 2] , [3 , 4] ] ").unwrap();
        assert_eq!(spaced, Matrix::from_ints(&[&[1, 2], &[3, 4]]).unwrap());
    }

    #[test]
    fn parses_flat_matrices() {
        let expected = Matrix::from_ints(&[&[1, 2], &[3, 4]]).unwrap();
        assert_eq!(parse_matrix("1 2; 3 4").unwrap(), expected);
        assert_eq!(parse_matrix("1 2\n3 4\n").unwrap(), expected);
        assert_eq!(parse_matrix("[1 2; 3 4]").unwrap(), expected);
        assert_eq!(parse_matrix("1 2 | 3 4;").unwrap(), expected);
        let frac = parse_matrix("1/2 0,5").unwrap();
        assert_eq!(frac[(0, 0)], frac[(0, 1)]);
    }

    #[test]
    fn rejects_malformed_matrices() {
        assert!(matches!(
            parse_matrix(""),
            Err(ParseError::Matrix(MatrixError::Empty))
        ));
        assert!(matches!(
            parse_matrix("[]"),
            Err(ParseError::Matrix(MatrixError::Empty))
        ));
        assert!(matches!(parse_matrix("[[]]"), Err(ParseError::Matrix(_))));
        assert!(matches!(
            parse_matrix("[[1,2],[3]]"),
            Err(ParseError::Matrix(MatrixError::RaggedRow { .. }))
        ));
        assert!(matches!(
            parse_matrix("[[1,2],[3,4]"),
            Err(ParseError::Malformed(_))
        ));
        assert!(matches!(
            parse_matrix("[[1,2]],[3,4]]"),
            Err(ParseError::Malformed(_))
        ));
        assert!(matches!(
            parse_matrix("[[1,[2]],[3,4]]"),
            Err(ParseError::Malformed(_))
        ));
        assert!(matches!(
            parse_matrix("1 2; 3 x"),
            Err(ParseError::Entry { row: 2, col: 2, .. })
        ));
        assert!(matches!(
            parse_matrix("1 2 3 4 5 6 7"),
            Err(ParseError::Matrix(MatrixError::TooLarge { .. }))
        ));
        assert!(matches!(
            parse_matrix("1;2;3;4;5;6;7"),
            Err(ParseError::Matrix(MatrixError::TooLarge { .. }))
        ));
        // In flat notation a comma is a decimal comma, so three comma-separated values are not a row.
        assert!(matches!(
            parse_matrix("1,2,3"),
            Err(ParseError::Entry { row: 1, col: 1, .. })
        ));
    }

    #[test]
    fn entry_errors_name_the_position() {
        let msg = parse_matrix("1 2; 3 x").unwrap_err().to_string();
        assert!(msg.starts_with("Eintrag in Zeile 2, Spalte 2:"), "{msg}");
    }

    #[test]
    fn parses_row_operations() {
        assert_eq!(
            parse_command("m 1 -1/10").unwrap(),
            Command::Operation(RowOperation::scale(0, ratio(-1, 10).unwrap()).unwrap())
        );
        assert_eq!(
            parse_command("  A 8 1 2 ").unwrap(),
            Command::Operation(RowOperation::add_multiple(int(8), 0, 1).unwrap())
        );
        assert_eq!(
            parse_command("swap 3 1").unwrap(),
            Command::Operation(RowOperation::swap(2, 0).unwrap())
        );
        assert_eq!(
            parse_command("mul 2 0.25").unwrap(),
            Command::Operation(RowOperation::scale(1, ratio(1, 4).unwrap()).unwrap())
        );
    }

    #[test]
    fn operation_commands_round_trip() {
        for op in [
            RowOperation::scale(2, ratio(-7, 3).unwrap()).unwrap(),
            RowOperation::add_multiple(ratio(-1, 2).unwrap(), 1, 0).unwrap(),
            RowOperation::swap(0, 4).unwrap(),
        ] {
            assert_eq!(
                parse_command(&op.to_command()).unwrap(),
                Command::Operation(op)
            );
        }
    }

    #[test]
    fn rejects_bad_operations() {
        assert!(matches!(
            parse_command("m 1 0"),
            Err(ParseError::Operation(OpError::ZeroFactor))
        ));
        assert!(matches!(
            parse_command("a 2 1 1"),
            Err(ParseError::Operation(OpError::AddToSelf { .. }))
        ));
        assert!(matches!(
            parse_command("s 2 2"),
            Err(ParseError::Operation(OpError::SwapWithSelf { .. }))
        ));
        assert!(matches!(
            parse_command("m 0 2"),
            Err(ParseError::InvalidIndex(_))
        ));
        assert!(matches!(
            parse_command("m -1 2"),
            Err(ParseError::InvalidIndex(_))
        ));
        assert!(matches!(
            parse_command("m 1.5 2"),
            Err(ParseError::InvalidIndex(_))
        ));
        assert!(matches!(
            parse_command("m 99999999999999999999 2"),
            Err(ParseError::InvalidIndex(_))
        ));
        assert!(matches!(
            parse_command("m 1"),
            Err(ParseError::Usage { .. })
        ));
        assert!(matches!(
            parse_command("a 1 2"),
            Err(ParseError::Usage { .. })
        ));
        assert!(matches!(
            parse_command("s 1 2 3"),
            Err(ParseError::Usage { .. })
        ));
        assert!(matches!(parse_command("m 1 x"), Err(ParseError::Number(_))));
        assert!(matches!(
            parse_command("a 1/0 1 2"),
            Err(ParseError::Number(NumberError::ZeroDenominator))
        ));
    }

    #[test]
    fn rejects_unknown_and_empty_commands() {
        assert_eq!(parse_command(""), Err(ParseError::EmptyInput));
        assert_eq!(parse_command("   "), Err(ParseError::EmptyInput));
        assert_eq!(
            parse_command("foo 1"),
            Err(ParseError::UnknownCommand("foo".into()))
        );
        assert!(matches!(
            parse_command("undo now"),
            Err(ParseError::UnexpectedArguments(_))
        ));
        let msg = parse_command("m 1").unwrap_err().to_string();
        assert!(msg.contains("m <i> <c>"), "{msg}");
    }

    #[test]
    fn parses_simple_commands_with_aliases() {
        let table = [
            ("u", Command::Undo),
            ("UNDO", Command::Undo),
            ("z", Command::Undo),
            ("r", Command::Redo),
            ("redo", Command::Redo),
            ("reset", Command::Reset),
            ("t", Command::Hint),
            ("tipp", Command::Hint),
            ("lösung", Command::Solution),
            ("loesung", Command::Solution),
            ("analyse", Command::ToggleAnalysis),
            ("exam", Command::ToggleExam),
            ("prüfung", Command::ToggleExam),
            ("theme", Command::ToggleTheme),
            ("v", Command::CycleView),
            ("?", Command::Help),
            ("hilfe", Command::Help),
            ("q", Command::Quit),
            ("exit", Command::Quit),
        ];
        for (input, expected) in table {
            assert_eq!(parse_command(input).unwrap(), expected, "{input}");
        }
    }

    #[test]
    fn parses_new_with_options() {
        assert_eq!(
            parse_command("n").unwrap(),
            Command::New(NewSpec::default())
        );
        let Command::New(spec) = parse_command("new 4x5 breit seed=7").unwrap() else {
            panic!("expected New");
        };
        assert_eq!(spec.size, Some((4, 5)));
        assert_eq!(spec.preset, Some(Preset::MoreColumns));
        assert_eq!(spec.seed, Some(7));
        let Command::New(spec) = parse_command("neu 2×3 #9 a11").unwrap() else {
            panic!("expected New");
        };
        assert_eq!(
            (spec.size, spec.seed, spec.preset),
            (Some((2, 3)), Some(9), Some(Preset::ZeroTopLeft))
        );
        assert!(matches!(
            parse_command("n 9x9"),
            Err(ParseError::Matrix(MatrixError::TooLarge { .. }))
        ));
        assert!(matches!(
            parse_command("n 0x3"),
            Err(ParseError::Matrix(MatrixError::Empty))
        ));
        assert!(matches!(
            parse_command("n 3x"),
            Err(ParseError::InvalidSize(_))
        ));
        assert!(matches!(
            parse_command("n seed=abc"),
            Err(ParseError::InvalidSeed(_))
        ));
        assert!(matches!(
            parse_command("n quark"),
            Err(ParseError::InvalidNewArgument(_))
        ));
    }

    #[test]
    fn parses_matrix_commands() {
        assert_eq!(parse_command("set").unwrap(), Command::SetMatrix(None));
        assert_eq!(parse_command("check").unwrap(), Command::Check(None));
        let expected = Matrix::from_ints(&[&[1, 2], &[3, 4]]).unwrap();
        assert_eq!(
            parse_command("set [[1,2],[3,4]]").unwrap(),
            Command::SetMatrix(Some(expected.clone()))
        );
        assert_eq!(
            parse_command("check 1 2; 3 4").unwrap(),
            Command::Check(Some(expected))
        );
        assert!(parse_command("set [[1,2],[3]]").is_err());
    }

    #[test]
    fn never_panics_on_odd_unicode() {
        for s in [
            "m ∞ ∞",
            "a \u{0}",
            "[[\u{202e}1]]",
            "s 1\t2",
            "   \n",
            "🦀",
            "[",
            "]",
            "[[",
            "]]",
            "[[]",
            "a - -",
        ] {
            let _ = parse_command(s);
            let _ = parse_matrix(s);
            let _ = parse_number(s);
        }
    }
}
