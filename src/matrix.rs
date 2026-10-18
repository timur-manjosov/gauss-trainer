//! Dense rational matrices of trainer size.

use std::fmt;
use std::ops::Index;

use thiserror::Error;

use crate::rational::{Q, int, to_text};
use num_traits::Zero;

/// Maximum number of rows and of columns of a [`Matrix`].
pub const MAX_DIM: usize = 6;

/// Reasons why a [`Matrix`] cannot be constructed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MatrixError {
    /// Zero rows or zero columns were requested.
    #[error("Eine Matrix braucht mindestens eine Zeile und eine Spalte.")]
    Empty,
    /// The dimensions exceed [`MAX_DIM`].
    #[error(
        "Die Matrix ist mit {rows}×{cols} zu groß – erlaubt sind höchstens {MAX_DIM}×{MAX_DIM}."
    )]
    TooLarge {
        /// Requested number of rows.
        rows: usize,
        /// Requested number of columns.
        cols: usize,
    },
    /// The number of entries does not match `rows * cols`.
    #[error("Es wurden {found} Einträge angegeben, erwartet werden {expected}.")]
    EntryCount {
        /// Required number of entries.
        expected: usize,
        /// Number of entries that were supplied.
        found: usize,
    },
    /// A row has a different length than the first row.
    #[error("Zeile {row} hat {found} Einträge, aber Zeile 1 hat {expected}.")]
    RaggedRow {
        /// One-based number of the offending row.
        row: usize,
        /// Length of the first row.
        expected: usize,
        /// Length of the offending row.
        found: usize,
    },
}

/// A rational `rows × cols` matrix stored row-major.
///
/// # Invariants
///
/// * `1 <= rows <= MAX_DIM` and `1 <= cols <= MAX_DIM`
/// * the entry buffer has exactly `rows * cols` elements
///
/// Rows and columns are **zero-based** in this API; only the user interface
/// and the command language count from one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    entries: Vec<Q>,
}

impl Matrix {
    /// Creates a matrix from row-major `entries`.
    ///
    /// # Errors
    ///
    /// Fails if a dimension is zero or larger than [`MAX_DIM`], or if
    /// `entries.len() != rows * cols`.
    pub fn new(rows: usize, cols: usize, entries: Vec<Q>) -> Result<Self, MatrixError> {
        if rows == 0 || cols == 0 {
            return Err(MatrixError::Empty);
        }
        if rows > MAX_DIM || cols > MAX_DIM {
            return Err(MatrixError::TooLarge { rows, cols });
        }
        if entries.len() != rows * cols {
            return Err(MatrixError::EntryCount {
                expected: rows * cols,
                found: entries.len(),
            });
        }
        Ok(Self {
            rows,
            cols,
            entries,
        })
    }

    /// Creates a matrix from a list of rows.
    ///
    /// # Errors
    ///
    /// Fails if there are no rows, if the rows differ in length, or if the
    /// size limits of [`Matrix::new`] are violated.
    pub fn from_rows(rows: Vec<Vec<Q>>) -> Result<Self, MatrixError> {
        let cols = rows.first().map_or(0, Vec::len);
        for (i, row) in rows.iter().enumerate() {
            if row.len() != cols {
                return Err(MatrixError::RaggedRow {
                    row: i + 1,
                    expected: cols,
                    found: row.len(),
                });
            }
        }
        let count = rows.len();
        Self::new(count, cols, rows.into_iter().flatten().collect())
    }

    /// Creates a matrix of small integers, mainly for tests and examples.
    ///
    /// # Errors
    ///
    /// Same conditions as [`Matrix::from_rows`].
    pub fn from_ints(rows: &[&[i64]]) -> Result<Self, MatrixError> {
        Self::from_rows(
            rows.iter()
                .map(|r| r.iter().map(|&n| int(n)).collect())
                .collect(),
        )
    }

    /// The 3×4 example matrix of the course, `[[1,-2,3,-1],[2,-1,2,2],[3,1,2,3]]`.
    ///
    /// Its Treppennormalform is `[[1,0,0,15/7],[0,1,0,-4/7],[0,0,1,-10/7]]`.
    #[must_use]
    pub fn course_example() -> Self {
        Self {
            rows: 3,
            cols: 4,
            entries: [1, -2, 3, -1, 2, -1, 2, 2, 3, 1, 2, 3].map(int).to_vec(),
        }
    }

    /// Number of rows.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns the entry at `(row, col)`, or `None` if out of range.
    #[must_use]
    pub fn get(&self, row: usize, col: usize) -> Option<&Q> {
        if row < self.rows && col < self.cols {
            self.entries.get(row * self.cols + col)
        } else {
            None
        }
    }

    /// Returns row `row` as a slice, or `None` if out of range.
    #[must_use]
    pub fn row(&self, row: usize) -> Option<&[Q]> {
        if row < self.rows {
            self.entries.get(row * self.cols..(row + 1) * self.cols)
        } else {
            None
        }
    }

    /// Iterates over the rows from top to bottom.
    pub fn iter_rows(&self) -> impl Iterator<Item = &[Q]> {
        self.entries.chunks(self.cols)
    }

    /// Returns `true` if every entry of `row` is zero (out-of-range rows count as zero).
    #[must_use]
    pub fn row_is_zero(&self, row: usize) -> bool {
        self.row(row).is_none_or(|r| r.iter().all(Zero::is_zero))
    }

    /// Returns the first column in which `row` is nonzero.
    #[must_use]
    pub fn leading_column(&self, row: usize) -> Option<usize> {
        self.row(row)?.iter().position(|q| !q.is_zero())
    }

    /// Multiplies row `row` by `factor` in place.
    ///
    /// Callers guarantee `row < rows`; see [`crate::ops::RowOperation::apply`]
    /// for the validated entry point.
    pub(crate) fn scale_row(&mut self, row: usize, factor: &Q) {
        let cols = self.cols;
        for q in &mut self.entries[row * cols..(row + 1) * cols] {
            *q *= factor;
        }
    }

    /// Adds `factor` times row `source` to row `target` in place.
    ///
    /// Callers guarantee both rows are in range.
    pub(crate) fn add_multiple(&mut self, factor: &Q, source: usize, target: usize) {
        let cols = self.cols;
        for j in 0..cols {
            let delta = &self.entries[source * cols + j] * factor;
            self.entries[target * cols + j] += delta;
        }
    }

    /// Swaps rows `a` and `b` in place.
    ///
    /// Callers guarantee both rows are in range.
    pub(crate) fn swap_rows(&mut self, a: usize, b: usize) {
        let cols = self.cols;
        for j in 0..cols {
            self.entries.swap(a * cols + j, b * cols + j);
        }
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = Q;

    /// Indexes by `(row, col)`.
    ///
    /// # Panics
    ///
    /// Panics if the position is out of range; use [`Matrix::get`] for a
    /// checked lookup.
    fn index(&self, (row, col): (usize, usize)) -> &Q {
        assert!(
            row < self.rows && col < self.cols,
            "matrix index out of range"
        );
        &self.entries[row * self.cols + col]
    }
}

impl fmt::Display for Matrix {
    /// Plain text with ASCII minus signs, one bracketed row per line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rows: Vec<String> = self
            .iter_rows()
            .map(|r| {
                let cells: Vec<String> = r.iter().map(to_text).collect();
                format!("[{}]", cells.join(", "))
            })
            .collect();
        write!(f, "[{}]", rows.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::ratio;

    fn sample() -> Matrix {
        Matrix::from_ints(&[&[1, 2, 3], &[4, 5, 6]]).unwrap()
    }

    #[test]
    fn rejects_bad_dimensions() {
        assert_eq!(Matrix::new(0, 3, vec![]), Err(MatrixError::Empty));
        assert_eq!(
            Matrix::new(7, 1, vec![int(0); 7]),
            Err(MatrixError::TooLarge { rows: 7, cols: 1 })
        );
        assert_eq!(
            Matrix::new(2, 2, vec![int(1)]),
            Err(MatrixError::EntryCount {
                expected: 4,
                found: 1
            })
        );
    }

    #[test]
    fn rejects_ragged_rows() {
        assert_eq!(
            Matrix::from_ints(&[&[1, 2], &[3]]),
            Err(MatrixError::RaggedRow {
                row: 2,
                expected: 2,
                found: 1
            })
        );
        assert_eq!(Matrix::from_rows(vec![]), Err(MatrixError::Empty));
    }

    #[test]
    fn accessors_are_checked() {
        let m = sample();
        assert_eq!((m.rows(), m.cols()), (2, 3));
        assert_eq!(m.get(1, 2), Some(&int(6)));
        assert_eq!(m.get(2, 0), None);
        assert_eq!(m.get(0, 3), None);
        assert_eq!(m.row(0).map(<[Q]>::len), Some(3));
        assert!(m.row(2).is_none());
        assert_eq!(m[(1, 0)], int(4));
    }

    #[test]
    fn finds_leading_columns() {
        let m = Matrix::from_ints(&[&[0, 0, 2], &[0, 0, 0]]).unwrap();
        assert_eq!(m.leading_column(0), Some(2));
        assert_eq!(m.leading_column(1), None);
        assert!(m.row_is_zero(1));
        assert!(!m.row_is_zero(0));
    }

    #[test]
    fn mutators_work_on_rows() {
        let mut m = sample();
        m.scale_row(0, &ratio(1, 2).unwrap());
        assert_eq!(m[(0, 1)], int(1));
        m.add_multiple(&int(-2), 0, 1);
        assert_eq!(m[(1, 0)], ratio(3, 1).unwrap());
        m.swap_rows(0, 1);
        assert_eq!(m[(0, 0)], int(3));
    }

    #[test]
    fn displays_ascii() {
        let m = Matrix::from_rows(vec![vec![int(1), ratio(-1, 2).unwrap()]]).unwrap();
        assert_eq!(m.to_string(), "[[1, -1/2]]");
    }
}
