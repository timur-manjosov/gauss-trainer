//! Elementary row operations.
//!
//! The three operations of the Gaussian algorithm, each of which is
//! invertible and therefore preserves the solution set and the rank:
//!
//! | Operation | Effect | Inverse |
//! |-----------|--------|---------|
//! | [`RowOperation::Scale`] | row *i* ← *c*·row *i*, *c* ≠ 0 | scale by 1/*c* |
//! | [`RowOperation::AddMultiple`] | row *j* ← row *j* + *c*·row *i*, *i* ≠ *j* | add (−*c*)-fold |
//! | [`RowOperation::Swap`] | rows *i* and *j* exchange places | itself |
//!
//! Row indices stored in an operation are **zero-based**; descriptions and
//! commands use the one-based numbering of the course.

use num_traits::Zero;
use thiserror::Error;

use crate::matrix::Matrix;
use crate::rational::{self, Q};

/// Reasons why a row operation is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpError {
    /// A row index lies outside the matrix.
    #[error("Zeile {row} gibt es nicht – die Matrix hat nur {}.", rows_label(.rows))]
    RowOutOfRange {
        /// The one-based row number that was requested.
        row: usize,
        /// Number of rows of the matrix.
        rows: usize,
    },
    /// A row was to be multiplied by zero.
    #[error(
        "Eine Zeile darf nicht mit 0 multipliziert werden – die Operation wäre nicht umkehrbar."
    )]
    ZeroFactor,
    /// A multiple of a row was to be added to the row itself.
    #[error("Zeile {row} kann nicht zu sich selbst addiert werden (es muss i ≠ j gelten).")]
    AddToSelf {
        /// The one-based row number.
        row: usize,
    },
    /// A row was to be swapped with itself.
    #[error("Zeile {row} mit sich selbst zu vertauschen ändert nichts.")]
    SwapWithSelf {
        /// The one-based row number.
        row: usize,
    },
}

fn rows_label(rows: &usize) -> String {
    if *rows == 1 {
        "1 Zeile".to_owned()
    } else {
        format!("{rows} Zeilen")
    }
}

/// The kind of a [`RowOperation`], without its operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// Multiply a row by a nonzero factor.
    Scale,
    /// Add a multiple of one row to another.
    AddMultiple,
    /// Exchange two rows.
    Swap,
}

/// An elementary row operation on a [`Matrix`].
///
/// Construct values with [`RowOperation::scale`], [`RowOperation::add_multiple`]
/// and [`RowOperation::swap`], which reject degenerate operands. The variants
/// are public for pattern matching; [`RowOperation::apply`] re-validates every
/// operation, so a hand-built invalid value can never corrupt a matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowOperation {
    /// Multiply row `row` by the nonzero `factor`.
    Scale {
        /// Zero-based row index.
        row: usize,
        /// Nonzero multiplier.
        factor: Q,
    },
    /// Add `factor` times row `source` to row `target` (`source != target`).
    AddMultiple {
        /// Multiplier applied to the source row.
        factor: Q,
        /// Zero-based index of the row that is multiplied.
        source: usize,
        /// Zero-based index of the row that changes.
        target: usize,
    },
    /// Exchange rows `first` and `second`.
    Swap {
        /// Zero-based index of the first row.
        first: usize,
        /// Zero-based index of the second row.
        second: usize,
    },
}

impl RowOperation {
    /// Builds "multiply row `row` by `factor`".
    ///
    /// # Errors
    ///
    /// [`OpError::ZeroFactor`] if `factor` is zero.
    pub fn scale(row: usize, factor: Q) -> Result<Self, OpError> {
        if factor.is_zero() {
            Err(OpError::ZeroFactor)
        } else {
            Ok(Self::Scale { row, factor })
        }
    }

    /// Builds "add `factor` times row `source` to row `target`".
    ///
    /// # Errors
    ///
    /// [`OpError::AddToSelf`] if `source == target`.
    pub fn add_multiple(factor: Q, source: usize, target: usize) -> Result<Self, OpError> {
        if source == target {
            Err(OpError::AddToSelf { row: source + 1 })
        } else {
            Ok(Self::AddMultiple {
                factor,
                source,
                target,
            })
        }
    }

    /// Builds "swap rows `first` and `second`".
    ///
    /// # Errors
    ///
    /// [`OpError::SwapWithSelf`] if both rows are the same.
    pub fn swap(first: usize, second: usize) -> Result<Self, OpError> {
        if first == second {
            Err(OpError::SwapWithSelf { row: first + 1 })
        } else {
            Ok(Self::Swap { first, second })
        }
    }

    /// The kind of this operation.
    #[must_use]
    pub fn kind(&self) -> OpKind {
        match self {
            Self::Scale { .. } => OpKind::Scale,
            Self::AddMultiple { .. } => OpKind::AddMultiple,
            Self::Swap { .. } => OpKind::Swap,
        }
    }

    /// Checks that the operation is legal for a matrix with `rows` rows.
    ///
    /// # Errors
    ///
    /// Reports out-of-range rows, a zero factor, or identical rows.
    pub fn validate(&self, rows: usize) -> Result<(), OpError> {
        let check = |row: usize| {
            if row < rows {
                Ok(())
            } else {
                Err(OpError::RowOutOfRange { row: row + 1, rows })
            }
        };
        match self {
            Self::Scale { row, factor } => {
                check(*row)?;
                if factor.is_zero() {
                    return Err(OpError::ZeroFactor);
                }
            }
            Self::AddMultiple { source, target, .. } => {
                check(*source)?;
                check(*target)?;
                if source == target {
                    return Err(OpError::AddToSelf { row: source + 1 });
                }
            }
            Self::Swap { first, second } => {
                check(*first)?;
                check(*second)?;
                if first == second {
                    return Err(OpError::SwapWithSelf { row: first + 1 });
                }
            }
        }
        Ok(())
    }

    /// Applies the operation to `matrix` in place.
    ///
    /// The operation is validated first, so on error the matrix is unchanged.
    /// Complexity: *O(n)* rational operations for *n* columns.
    ///
    /// # Errors
    ///
    /// See [`RowOperation::validate`].
    pub fn apply(&self, matrix: &mut Matrix) -> Result<(), OpError> {
        self.validate(matrix.rows())?;
        match self {
            Self::Scale { row, factor } => matrix.scale_row(*row, factor),
            Self::AddMultiple {
                factor,
                source,
                target,
            } => {
                matrix.add_multiple(factor, *source, *target);
            }
            Self::Swap { first, second } => matrix.swap_rows(*first, *second),
        }
        Ok(())
    }

    /// Returns a copy of `matrix` with the operation applied.
    ///
    /// # Errors
    ///
    /// See [`RowOperation::validate`].
    pub fn applied_to(&self, matrix: &Matrix) -> Result<Matrix, OpError> {
        let mut copy = matrix.clone();
        self.apply(&mut copy)?;
        Ok(copy)
    }

    /// Returns the operation that undoes this one.
    ///
    /// For every valid operation `op` and matrix `m`, applying `op` and then
    /// `op.inverse()` restores `m` exactly. An operation with a zero factor
    /// (which [`apply`](Self::apply) rejects) is returned unchanged.
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            Self::Scale { row, factor } => match rational::inverse(factor) {
                Some(inv) => Self::Scale {
                    row: *row,
                    factor: inv,
                },
                None => self.clone(),
            },
            Self::AddMultiple {
                factor,
                source,
                target,
            } => Self::AddMultiple {
                factor: -factor,
                source: *source,
                target: *target,
            },
            Self::Swap { .. } => self.clone(),
        }
    }

    /// Zero-based indices of the rows whose content this operation may change.
    #[must_use]
    pub fn touched_rows(&self) -> Vec<usize> {
        match self {
            Self::Scale { row, .. } => vec![*row],
            Self::AddMultiple { target, .. } => vec![*target],
            Self::Swap { first, second } => vec![*first, *second],
        }
    }

    /// Describes the operation in the wording of the Maple trainer.
    ///
    /// ```
    /// use gauss_trainer::ops::RowOperation;
    /// use gauss_trainer::rational::int;
    ///
    /// let op = RowOperation::add_multiple(int(8), 0, 1).unwrap();
    /// assert_eq!(op.describe(), "Das 8-fache von Zeile 1 wird zu Zeile 2 addiert.");
    /// ```
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Scale { row, factor } => format!(
                "Zeile {} wird mit dem Faktor {} multipliziert.",
                row + 1,
                rational::to_text(factor)
            ),
            Self::AddMultiple {
                factor,
                source,
                target,
            } => {
                let text = rational::to_text(factor);
                let shown = if text.starts_with('-') {
                    format!("({text})")
                } else {
                    text
                };
                format!(
                    "Das {shown}-fache von Zeile {} wird zu Zeile {} addiert.",
                    source + 1,
                    target + 1
                )
            }
            Self::Swap { first, second } => {
                format!(
                    "Zeile {} wird mit Zeile {} vertauscht.",
                    first + 1,
                    second + 1
                )
            }
        }
    }

    /// The command that enters this operation, e.g. `a -2 1 2`.
    ///
    /// The result parses back to an equal operation with
    /// [`crate::parse::parse_command`].
    #[must_use]
    pub fn to_command(&self) -> String {
        match self {
            Self::Scale { row, factor } => format!("m {} {}", row + 1, rational::to_text(factor)),
            Self::AddMultiple {
                factor,
                source,
                target,
            } => {
                format!(
                    "a {} {} {}",
                    rational::to_text(factor),
                    source + 1,
                    target + 1
                )
            }
            Self::Swap { first, second } => format!("s {} {}", first + 1, second + 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::{int, ratio};

    fn sample() -> Matrix {
        Matrix::from_ints(&[&[1, 2], &[3, 4], &[5, 6]]).unwrap()
    }

    #[test]
    fn constructors_reject_degenerate_operands() {
        assert_eq!(RowOperation::scale(0, int(0)), Err(OpError::ZeroFactor));
        assert_eq!(
            RowOperation::add_multiple(int(1), 1, 1),
            Err(OpError::AddToSelf { row: 2 })
        );
        assert_eq!(
            RowOperation::swap(2, 2),
            Err(OpError::SwapWithSelf { row: 3 })
        );
    }

    #[test]
    fn applies_all_three_operations() {
        let mut m = sample();
        RowOperation::scale(0, ratio(1, 2).unwrap())
            .unwrap()
            .apply(&mut m)
            .unwrap();
        assert_eq!(m[(0, 0)], ratio(1, 2).unwrap());
        RowOperation::add_multiple(int(-6), 0, 1)
            .unwrap()
            .apply(&mut m)
            .unwrap();
        assert_eq!(m[(1, 0)], int(0));
        RowOperation::swap(0, 2).unwrap().apply(&mut m).unwrap();
        assert_eq!(m[(0, 0)], int(5));
    }

    #[test]
    fn out_of_range_leaves_matrix_untouched() {
        let mut m = sample();
        let before = m.clone();
        let op = RowOperation::Swap {
            first: 0,
            second: 3,
        };
        assert_eq!(
            op.apply(&mut m),
            Err(OpError::RowOutOfRange { row: 4, rows: 3 })
        );
        assert_eq!(m, before);
    }

    #[test]
    fn hand_built_invalid_operations_are_rejected() {
        let mut m = sample();
        let zero = RowOperation::Scale {
            row: 0,
            factor: int(0),
        };
        assert_eq!(zero.apply(&mut m), Err(OpError::ZeroFactor));
        let same = RowOperation::AddMultiple {
            factor: int(1),
            source: 1,
            target: 1,
        };
        assert_eq!(same.apply(&mut m), Err(OpError::AddToSelf { row: 2 }));
    }

    #[test]
    fn inverse_restores_the_matrix() {
        let ops = [
            RowOperation::scale(1, ratio(-3, 7).unwrap()).unwrap(),
            RowOperation::add_multiple(ratio(5, 3).unwrap(), 2, 0).unwrap(),
            RowOperation::swap(0, 1).unwrap(),
        ];
        for op in ops {
            let m = sample();
            let mut changed = op.applied_to(&m).unwrap();
            assert_ne!(changed, m);
            op.inverse().apply(&mut changed).unwrap();
            assert_eq!(changed, m);
        }
    }

    #[test]
    fn describes_in_maple_wording() {
        assert_eq!(
            RowOperation::scale(0, ratio(-1, 10).unwrap())
                .unwrap()
                .describe(),
            "Zeile 1 wird mit dem Faktor -1/10 multipliziert."
        );
        assert_eq!(
            RowOperation::add_multiple(int(8), 0, 1).unwrap().describe(),
            "Das 8-fache von Zeile 1 wird zu Zeile 2 addiert."
        );
        assert_eq!(
            RowOperation::add_multiple(int(-2), 2, 0)
                .unwrap()
                .describe(),
            "Das (-2)-fache von Zeile 3 wird zu Zeile 1 addiert."
        );
        assert_eq!(
            RowOperation::swap(0, 1).unwrap().describe(),
            "Zeile 1 wird mit Zeile 2 vertauscht."
        );
    }

    #[test]
    fn renders_commands() {
        assert_eq!(
            RowOperation::scale(0, ratio(1, 2).unwrap())
                .unwrap()
                .to_command(),
            "m 1 1/2"
        );
        assert_eq!(
            RowOperation::add_multiple(int(-3), 1, 2)
                .unwrap()
                .to_command(),
            "a -3 2 3"
        );
        assert_eq!(RowOperation::swap(0, 2).unwrap().to_command(), "s 1 3");
    }

    #[test]
    fn reports_touched_rows() {
        assert_eq!(RowOperation::swap(0, 2).unwrap().touched_rows(), vec![0, 2]);
        assert_eq!(
            RowOperation::add_multiple(int(1), 0, 2)
                .unwrap()
                .touched_rows(),
            vec![2]
        );
    }

    #[test]
    fn error_messages_are_german_and_pluralised() {
        let e = OpError::RowOutOfRange { row: 5, rows: 1 };
        assert_eq!(
            e.to_string(),
            "Zeile 5 gibt es nicht – die Matrix hat nur 1 Zeile."
        );
        let e = OpError::RowOutOfRange { row: 5, rows: 3 };
        assert_eq!(
            e.to_string(),
            "Zeile 5 gibt es nicht – die Matrix hat nur 3 Zeilen."
        );
    }
}
