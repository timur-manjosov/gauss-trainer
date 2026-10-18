//! Check mode: compare a hand-computed matrix with the current matrix and
//! with the unique Treppennormalform.

use crate::echelon::rref;
use crate::matrix::Matrix;
use crate::rational::Q;

/// One entry in which the student's matrix differs from a reference matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    /// Zero-based row.
    pub row: usize,
    /// Zero-based column.
    pub col: usize,
    /// What the student entered.
    pub entered: Q,
    /// What the reference matrix contains.
    pub expected: Q,
}

/// Result of comparing an entered matrix with the trainer's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// Size of the matrix the student entered, as `(rows, cols)`.
    pub entered_size: (usize, usize),
    /// Size of the trainer's matrix, as `(rows, cols)`.
    pub expected_size: (usize, usize),
    /// The entered matrix equals the current matrix.
    pub matches_current: bool,
    /// The entered matrix equals the Treppennormalform.
    pub matches_rref: bool,
    /// Entries that differ from the current matrix (empty on a size mismatch).
    pub diff_current: Vec<Diff>,
    /// Entries that differ from the Treppennormalform (empty on a size mismatch).
    pub diff_rref: Vec<Diff>,
}

impl CheckReport {
    /// Whether the entered matrix has the same size as the current one.
    #[must_use]
    pub fn sizes_match(&self) -> bool {
        self.entered_size == self.expected_size
    }
}

/// Compares `entered` with `current` and with the Treppennormalform of `current`.
///
/// Row operations preserve the Treppennormalform, so it is the same for
/// every matrix in the history. Complexity: one solve plus *O(m·n)*.
#[must_use]
pub fn check(entered: &Matrix, current: &Matrix) -> CheckReport {
    let target = rref(current);
    let same_size = entered.rows() == current.rows() && entered.cols() == current.cols();
    let diff_against = |reference: &Matrix| -> Vec<Diff> {
        if !same_size {
            return Vec::new();
        }
        let mut diffs = Vec::new();
        for i in 0..entered.rows() {
            for j in 0..entered.cols() {
                if let (Some(a), Some(b)) = (entered.get(i, j), reference.get(i, j))
                    && a != b
                {
                    diffs.push(Diff {
                        row: i,
                        col: j,
                        entered: a.clone(),
                        expected: b.clone(),
                    });
                }
            }
        }
        diffs
    };
    let diff_current = diff_against(current);
    let diff_rref = diff_against(&target);
    CheckReport {
        entered_size: (entered.rows(), entered.cols()),
        expected_size: (current.rows(), current.cols()),
        matches_current: same_size && diff_current.is_empty(),
        matches_rref: same_size && diff_rref.is_empty(),
        diff_current,
        diff_rref,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::int;

    fn m(rows: &[&[i64]]) -> Matrix {
        Matrix::from_ints(rows).unwrap()
    }

    #[test]
    fn recognises_the_current_matrix() {
        let a = m(&[&[1, 2], &[3, 4]]);
        let r = check(&a, &a);
        assert!(r.matches_current);
        assert!(!r.matches_rref);
        assert!(r.diff_current.is_empty());
        assert_eq!(r.diff_rref.len(), 3);
    }

    #[test]
    fn recognises_the_treppennormalform() {
        let a = m(&[&[1, 2], &[3, 4]]);
        let r = check(&m(&[&[1, 0], &[0, 1]]), &a);
        assert!(r.matches_rref);
        assert!(!r.matches_current);
    }

    #[test]
    fn lists_differing_entries() {
        let a = m(&[&[1, 2], &[3, 4]]);
        let r = check(&m(&[&[1, 5], &[3, 4]]), &a);
        assert_eq!(
            r.diff_current,
            vec![Diff {
                row: 0,
                col: 1,
                entered: int(5),
                expected: int(2)
            }]
        );
        assert!(!r.matches_current && !r.matches_rref);
    }

    #[test]
    fn reports_size_mismatches() {
        let a = m(&[&[1, 2], &[3, 4]]);
        let r = check(&m(&[&[1, 2, 3]]), &a);
        assert!(!r.sizes_match());
        assert_eq!(r.entered_size, (1, 3));
        assert_eq!(r.expected_size, (2, 2));
        assert!(!r.matches_current && !r.matches_rref);
        assert!(r.diff_current.is_empty());
    }
}
