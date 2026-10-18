//! Command-line arguments.
//!
//! A hand-written parser: the option set is tiny, and this keeps the
//! dependency tree small.

use thiserror::Error;

use crate::app::{AppConfig, Theme};
use crate::generator::Preset;
use crate::matrix::{MAX_DIM, Matrix};
use crate::parse::{ParseError, parse_matrix};
use crate::ui::matrix_view::Brackets;
use crate::ui::palette::ColorDepth;

/// The usage text printed for `--help`.
pub const HELP: &str = "\
gauss-trainer – Übungsprogramm zum Gaußschen Algorithmus (Gauß-Jordan-Verfahren)

Aufruf:
  gauss-trainer [OPTIONEN] [MATRIX]

MATRIX ist eine eigene Startmatrix, z. B. \"[[1,-2,3,-1],[2,-1,2,2],[3,1,2,3]]\"
oder \"1 -2 3 -1; 2 -1 2 2; 3 1 2 3\". Ohne Angabe wird eine Aufgabe erzeugt.
Beginnt die Matrix mit einem Minuszeichen, verwende --matrix oder setze -- davor.

Optionen:
  -s, --seed <N>        Seed für reproduzierbare Aufgaben
      --size <MxN>      Größe der erzeugten Matrix, 1..=6 je Richtung (Standard 3x4)
  -p, --preset <ART>    allgemein | a11 | abhängig | breit | hoch
      --matrix <MATRIX> eigene Startmatrix (wie das Argument MATRIX)
      --theme <NAME>    dunkel (Standard) | hell
      --exam            direkt im Prüfungsmodus starten
      --no-live         Live-Analyse zu Beginn ausschalten
      --colors <MODUS>  auto (Standard) | truecolor | 256
      --simple-brackets Klammern aus Rahmenzeichen (┌ │ └) statt ⎡ ⎢ ⎣
  -h, --help            diese Hilfe
  -V, --version         Versionsnummer

Im Programm zeigt ? alle Befehle und Tastenkürzel.
";

/// Problems with the command line.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CliError {
    /// An option that does not exist.
    #[error("Unbekannte Option »{0}« – mit --help siehst du alle Optionen.")]
    UnknownOption(String),
    /// An option that needs a value was last.
    #[error("Die Option {0} braucht einen Wert.")]
    MissingValue(String),
    /// A value that could not be understood.
    #[error("Ungültiger Wert »{value}« für {option}: {reason}")]
    InvalidValue {
        /// The option.
        option: String,
        /// The offending value.
        value: String,
        /// Why it was rejected.
        reason: String,
    },
    /// The start matrix could not be parsed.
    #[error("Startmatrix: {0}")]
    Matrix(#[from] ParseError),
    /// A non-unicode argument.
    #[error("Die Argumente müssen gültiges Unicode sein.")]
    NotUnicode,
}

/// Everything the binary needs to start.
#[derive(Debug, Clone)]
pub struct Options {
    /// Application configuration (the seed is only a placeholder when
    /// [`Options::seed`] is `None`).
    pub config: AppConfig,
    /// The seed given on the command line; `None` asks for a fresh one.
    pub seed: Option<u64>,
    /// Forced colour depth; `None` autodetects.
    pub depth: Option<ColorDepth>,
    /// Bracket glyphs.
    pub brackets: Brackets,
}

/// What the program should do.
#[derive(Debug, Clone)]
pub enum Invocation {
    /// Start the trainer.
    Run(Box<Options>),
    /// Print [`HELP`].
    Help,
    /// Print the version.
    Version,
}

fn invalid(option: &str, value: &str, reason: &str) -> CliError {
    CliError::InvalidValue {
        option: option.to_owned(),
        value: value.to_owned(),
        reason: reason.to_owned(),
    }
}

fn parse_size(option: &str, value: &str) -> Result<(usize, usize), CliError> {
    let reason = format!("erwartet wird <Zeilen>x<Spalten> mit Werten von 1 bis {MAX_DIM}");
    let (r, c) = value
        .split_once(['x', '×', 'X'])
        .ok_or_else(|| invalid(option, value, &reason))?;
    let parse = |s: &str| {
        s.parse::<usize>()
            .ok()
            .filter(|n| (1..=MAX_DIM).contains(n))
    };
    match (parse(r), parse(c)) {
        (Some(rows), Some(cols)) => Ok((rows, cols)),
        _ => Err(invalid(option, value, &reason)),
    }
}

/// Parses the arguments (without the program name).
///
/// # Errors
///
/// Returns a [`CliError`] with a German message for unknown options,
/// missing or invalid values and malformed matrices.
pub fn parse_args<I>(args: I) -> Result<Invocation, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut config = AppConfig::default();
    let mut seed = None;
    let mut depth = None;
    let mut brackets = Brackets::Tall;
    let mut matrix_text: Vec<String> = Vec::new();
    let mut options_done = false;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if options_done || !arg.starts_with('-') || arg.len() == 1 {
            matrix_text.push(arg);
            continue;
        }
        // Allow --option=value.
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n.to_owned(), Some(v.to_owned())),
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| -> Result<String, CliError> {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| CliError::MissingValue(name.to_owned()))
        };
        // A negative number or a matrix may follow --matrix, so take its value verbatim.
        match name.as_str() {
            "--" => options_done = true,
            "-h" | "--help" => return Ok(Invocation::Help),
            "-V" | "--version" => return Ok(Invocation::Version),
            "-s" | "--seed" => {
                let v = value(&name)?;
                seed = Some(
                    v.parse::<u64>()
                        .map_err(|_| invalid(&name, &v, "erwartet wird eine ganze Zahl ≥ 0"))?,
                );
            }
            "--size" => {
                let v = value(&name)?;
                (config.rows, config.cols) = parse_size(&name, &v)?;
            }
            "-p" | "--preset" => {
                let v = value(&name)?;
                config.preset = Preset::from_keyword(&v).ok_or_else(|| {
                    invalid(
                        &name,
                        &v,
                        "erlaubt sind allgemein, a11, abhängig, breit, hoch",
                    )
                })?;
            }
            "--matrix" => matrix_text.push(value(&name)?),
            "--theme" => {
                let v = value(&name)?;
                config.theme = match v.to_lowercase().as_str() {
                    "dunkel" | "dark" => Theme::Dark,
                    "hell" | "light" => Theme::Light,
                    _ => return Err(invalid(&name, &v, "erlaubt sind dunkel und hell")),
                };
            }
            "--exam" => config.exam = true,
            "--no-live" => config.live_analysis = false,
            "--colors" => {
                let v = value(&name)?;
                depth = match v.to_lowercase().as_str() {
                    "auto" => None,
                    "truecolor" | "24bit" => Some(ColorDepth::TrueColor),
                    "256" => Some(ColorDepth::Indexed256),
                    _ => return Err(invalid(&name, &v, "erlaubt sind auto, truecolor und 256")),
                };
            }
            "--simple-brackets" => brackets = Brackets::Simple,
            _ => return Err(CliError::UnknownOption(arg)),
        }
    }

    if !matrix_text.is_empty() {
        let matrix: Matrix = parse_matrix(&matrix_text.join(" "))?;
        config.matrix = Some(matrix);
    }
    if let Some(s) = seed {
        config.seed = s;
    }
    Ok(Invocation::Run(Box::new(Options {
        config,
        seed,
        depth,
        brackets,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::int;

    fn parse(args: &[&str]) -> Result<Invocation, CliError> {
        parse_args(args.iter().map(|s| (*s).to_owned()))
    }

    fn options(args: &[&str]) -> Options {
        match parse(args).unwrap() {
            Invocation::Run(o) => *o,
            other => panic!("expected Run, got {other:?}"),
        }
    }

    #[test]
    fn defaults() {
        let o = options(&[]);
        assert_eq!((o.config.rows, o.config.cols), (3, 4));
        assert_eq!(o.seed, None);
        assert!(o.config.matrix.is_none());
        assert!(o.config.live_analysis && !o.config.exam);
        assert_eq!(o.config.theme, Theme::Dark);
    }

    #[test]
    fn parses_options() {
        let o = options(&[
            "--seed",
            "42",
            "--size",
            "2x5",
            "-p",
            "breit",
            "--theme=hell",
            "--exam",
            "--no-live",
            "--colors",
            "256",
            "--simple-brackets",
        ]);
        assert_eq!(o.seed, Some(42));
        assert_eq!(o.config.seed, 42);
        assert_eq!((o.config.rows, o.config.cols), (2, 5));
        assert_eq!(o.config.preset, Preset::MoreColumns);
        assert_eq!(o.config.theme, Theme::Light);
        assert!(o.config.exam && !o.config.live_analysis);
        assert_eq!(o.depth, Some(ColorDepth::Indexed256));
        assert_eq!(o.brackets, Brackets::Simple);
    }

    #[test]
    fn positional_matrix_arguments() {
        let o = options(&["[[1,-2],[3,4]]"]);
        assert_eq!(o.config.matrix.unwrap()[(0, 1)], int(-2));
        let o = options(&["1", "2", ";", "3", "4"]);
        assert_eq!(o.config.matrix.unwrap().rows(), 2);
        let o = options(&["--matrix", "-1 2; 3 4"]);
        assert_eq!(o.config.matrix.unwrap()[(0, 0)], int(-1));
        let o = options(&["--", "-1 2; 3 4"]);
        assert_eq!(o.config.matrix.unwrap()[(0, 0)], int(-1));
    }

    #[test]
    fn help_and_version() {
        assert!(matches!(parse(&["--help"]).unwrap(), Invocation::Help));
        assert!(matches!(parse(&["-V"]).unwrap(), Invocation::Version));
        assert!(matches!(
            parse(&["--seed", "1", "-h"]).unwrap(),
            Invocation::Help
        ));
    }

    #[test]
    fn reports_errors_in_german() {
        assert!(matches!(
            parse(&["--bogus"]),
            Err(CliError::UnknownOption(_))
        ));
        assert!(matches!(parse(&["--seed"]), Err(CliError::MissingValue(_))));
        assert!(matches!(
            parse(&["--seed", "abc"]),
            Err(CliError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse(&["--seed", "-3"]),
            Err(CliError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse(&["--size", "7x2"]),
            Err(CliError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse(&["--size", "3"]),
            Err(CliError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse(&["--preset", "xyz"]),
            Err(CliError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse(&["--theme", "bunt"]),
            Err(CliError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse(&["--colors", "16"]),
            Err(CliError::InvalidValue { .. })
        ));
        assert!(matches!(parse(&["[[1,2],[3]]"]), Err(CliError::Matrix(_))));
        let msg = parse(&["--size", "9x9"]).unwrap_err().to_string();
        assert!(msg.contains("Ungültiger Wert »9x9« für --size"), "{msg}");
    }

    #[test]
    fn help_text_mentions_every_option() {
        for option in [
            "--seed",
            "--size",
            "--preset",
            "--matrix",
            "--theme",
            "--exam",
            "--no-live",
            "--colors",
            "--simple-brackets",
            "--help",
            "--version",
        ] {
            assert!(HELP.contains(option), "{option}");
        }
    }
}
