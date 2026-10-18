//! Escalating hints derived from the reference solver.
//!
//! Hints are computed from the *current* matrix, not from the initial one, so
//! they stay correct however the student got there. They are only produced on
//! request; the application never calls this module unprompted.

use crate::echelon::{Purpose, Step, next_step};
use crate::matrix::Matrix;

/// The highest hint level.
pub const MAX_LEVEL: u8 = 3;

/// A hint of a given level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    /// The level, `1..=MAX_LEVEL`.
    pub level: u8,
    /// The German hint text.
    pub text: String,
}

/// Produces the hint of `level` (clamped to `1..=MAX_LEVEL`) for `matrix`.
///
/// * Level 1 asks a guiding question and reveals nothing about the next step.
/// * Level 2 names the column and the *kind* of operation.
/// * Level 3 gives the exact operation and the command that enters it.
#[must_use]
pub fn hint(level: u8, matrix: &Matrix) -> Hint {
    let level = level.clamp(1, MAX_LEVEL);
    let text = match next_step(matrix) {
        None => "Die Matrix steht bereits in Treppennormalform – hier ist nichts mehr zu tun.".to_owned(),
        Some(step) => match level {
            1 => "Welche Spalte ist als Nächstes dran – und welcher Eintrag soll dort zur Pivotstelle \
                  werden? Ist er schon 1, und stehen in der Spalte sonst nur Nullen?"
                .to_owned(),
            2 => kind_hint(&step),
            _ => format!("Gib ein: {}   ({})", step.op.to_command(), step.op.describe()),
        },
    };
    Hint { level, text }
}

fn kind_hint(step: &Step) -> String {
    let col = step.column + 1;
    let row = step.pivot_row + 1;
    match step.purpose {
        Purpose::MovePivot => format!(
            "Spalte {col}: In Zeile {row} steht noch kein passender Eintrag für die Pivotstelle. \
             Hier hilft eine Zeilenvertauschung."
        ),
        Purpose::Normalise => format!(
            "Spalte {col}: Der Pivot in Zeile {row} ist noch nicht 1. \
             Hier hilft das Multiplizieren einer Zeile mit einem Faktor."
        ),
        Purpose::Eliminate => format!(
            "Spalte {col}: Über oder unter dem Pivot in Zeile {row} steht noch ein Eintrag ≠ 0. \
             Hier hilft das Addieren eines Vielfachen einer Zeile zu einer anderen."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(rows: &[&[i64]]) -> Matrix {
        Matrix::from_ints(rows).unwrap()
    }

    #[test]
    fn level_one_is_a_question_without_spoilers() {
        let h = hint(1, &m(&[&[0, 1], &[1, 0]]));
        assert_eq!(h.level, 1);
        assert!(h.text.contains('?'));
        assert!(!h.text.contains("Zeilenvertauschung"));
        assert!(!h.text.contains("s 1 2"));
    }

    #[test]
    fn level_two_names_the_kind_of_operation() {
        let swap = hint(2, &m(&[&[0, 1], &[1, 0]]));
        assert!(swap.text.contains("Zeilenvertauschung"), "{}", swap.text);
        let scale = hint(2, &m(&[&[2, 1], &[0, 1]]));
        assert!(scale.text.contains("Multiplizieren"), "{}", scale.text);
        let add = hint(2, &m(&[&[1, 1], &[3, 1]]));
        assert!(add.text.contains("Addieren"), "{}", add.text);
        assert!(add.text.contains("Spalte 1"));
    }

    #[test]
    fn level_three_gives_the_exact_command() {
        let h = hint(3, &m(&[&[1, 1], &[3, 1]]));
        assert!(h.text.contains("a -3 1 2"), "{}", h.text);
        assert!(
            h.text
                .contains("Das (-3)-fache von Zeile 1 wird zu Zeile 2 addiert.")
        );
    }

    #[test]
    fn finished_matrices_get_a_congratulation_at_every_level() {
        for level in 1..=MAX_LEVEL {
            let h = hint(level, &m(&[&[1, 0], &[0, 1]]));
            assert!(h.text.contains("bereits in Treppennormalform"));
        }
    }

    #[test]
    fn levels_are_clamped() {
        assert_eq!(hint(0, &m(&[&[0, 1], &[1, 0]])).level, 1);
        assert_eq!(hint(9, &m(&[&[0, 1], &[1, 0]])).level, MAX_LEVEL);
    }
}
