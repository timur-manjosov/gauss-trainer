//! Analysis of echelon structure, rank, and the reference solver.
//!
//! # Definition
//!
//! A matrix is in **Treppennormalform** (reduced row echelon form, RREF) iff
//!
//! 1. all zero rows are at the bottom,
//! 2. the first nonzero entry of every nonzero row (its *pivot*) equals 1,
//! 3. the pivot columns of consecutive nonzero rows strictly increase, and
//! 4. every pivot column contains zeros except for its pivot.
//!
//! Every matrix can be brought into this form by elementary row operations,
//! and the result is **unique** (see the README for a proof sketch). The rank
//! of a matrix equals the number of nonzero rows of that form.

use num_traits::{One, Zero};

use crate::matrix::Matrix;
use crate::ops::{OpKind, RowOperation};
use crate::rational::{self, Q};

/// A pivot: the first nonzero entry of a row, provided no row above already
/// has its pivot in the same column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pivot {
    /// Zero-based row.
    pub row: usize,
    /// Zero-based column.
    pub col: usize,
    /// Whether the entry already equals 1.
    pub is_one: bool,
}

/// What the analysis says about a single matrix cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellRole {
    /// A pivot (see [`Pivot`]).
    Pivot {
        /// Whether the entry already equals 1.
        is_one: bool,
    },
    /// A nonzero entry in a pivot column that is not the pivot of that column
    /// and therefore still has to become 0.
    ToClear,
    /// Any other cell.
    Plain,
}

/// A reason why a matrix is not yet in Treppennormalform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    /// A zero row stands above a nonzero row.
    ZeroRowAboveNonzero {
        /// Zero-based index of the zero row.
        row: usize,
    },
    /// A pivot lies in an earlier column than the pivot of the row above.
    PivotOrder {
        /// Zero-based index of the row whose pivot is too far left.
        row: usize,
    },
    /// A pivot is not 1.
    PivotNotOne {
        /// Zero-based row of the pivot.
        row: usize,
        /// Zero-based column of the pivot.
        col: usize,
    },
    /// A pivot column still contains a nonzero entry besides its pivot.
    NotCleared {
        /// Zero-based row of the entry.
        row: usize,
        /// Zero-based column of the entry.
        col: usize,
    },
}

/// Structural analysis of a matrix, computed in *O(m·n)* plus one solve for the rank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    rows: usize,
    cols: usize,
    rank: usize,
    pivots: Vec<Pivot>,
    zero_rows: Vec<usize>,
    pivot_cols: Vec<usize>,
    to_clear: Vec<(usize, usize)>,
    issues: Vec<Issue>,
}

impl Analysis {
    /// Analyses `m`.
    #[must_use]
    pub fn new(m: &Matrix) -> Self {
        // A row's first nonzero entry is a pivot only if no row above already has its
        // pivot in that column; otherwise that entry still has to become 0.
        let mut pivots: Vec<Pivot> = Vec::new();
        let mut zero_rows = Vec::new();
        for i in 0..m.rows() {
            match m.leading_column(i) {
                Some(col) if pivots.iter().all(|p| p.col != col) => {
                    let is_one = m.get(i, col).is_some_and(rational::is_one);
                    pivots.push(Pivot {
                        row: i,
                        col,
                        is_one,
                    });
                }
                Some(_) => {}
                None => zero_rows.push(i),
            }
        }

        let mut pivot_cols: Vec<usize> = pivots.iter().map(|p| p.col).collect();
        pivot_cols.sort_unstable();

        let mut to_clear = Vec::new();
        for p in &pivots {
            for i in 0..m.rows() {
                if i != p.row && m.get(i, p.col).is_some_and(|q| !q.is_zero()) {
                    to_clear.push((i, p.col));
                }
            }
        }
        to_clear.sort_unstable();

        let mut issues = Vec::new();
        if let Some(last) = (0..m.rows()).rev().find(|&i| !m.row_is_zero(i)) {
            issues.extend(
                zero_rows
                    .iter()
                    .filter(|&&z| z < last)
                    .map(|&row| Issue::ZeroRowAboveNonzero { row }),
            );
        }
        for pair in pivots.windows(2) {
            if pair[1].col < pair[0].col {
                issues.push(Issue::PivotOrder { row: pair[1].row });
            }
        }
        for p in &pivots {
            if !p.is_one {
                issues.push(Issue::PivotNotOne {
                    row: p.row,
                    col: p.col,
                });
            }
        }
        issues.extend(
            to_clear
                .iter()
                .map(|&(row, col)| Issue::NotCleared { row, col }),
        );

        Self {
            rows: m.rows(),
            cols: m.cols(),
            rank: rank(m),
            pivots,
            zero_rows,
            pivot_cols,
            to_clear,
            issues,
        }
    }

    /// The rank *r* of the matrix (invariant under row operations).
    #[must_use]
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// The pivots, top to bottom: one per pivot column (see [`Pivot`]).
    #[must_use]
    pub fn pivots(&self) -> &[Pivot] {
        &self.pivots
    }

    /// Zero-based indices of the zero rows.
    #[must_use]
    pub fn zero_rows(&self) -> &[usize] {
        &self.zero_rows
    }

    /// Sorted, distinct zero-based columns that contain a pivot.
    #[must_use]
    pub fn pivot_cols(&self) -> &[usize] {
        &self.pivot_cols
    }

    /// Entries (row, col) that lie in a pivot column and still have to become 0.
    #[must_use]
    pub fn to_clear(&self) -> &[(usize, usize)] {
        &self.to_clear
    }

    /// All reasons why the matrix is not yet in Treppennormalform.
    #[must_use]
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Whether the matrix is in Treppennormalform.
    #[must_use]
    pub fn is_rref(&self) -> bool {
        self.issues.is_empty()
    }

    /// Whether column `col` contains a pivot.
    #[must_use]
    pub fn is_pivot_col(&self, col: usize) -> bool {
        self.pivot_cols.binary_search(&col).is_ok()
    }

    /// Whether row `row` is a zero row.
    #[must_use]
    pub fn is_zero_row(&self, row: usize) -> bool {
        self.zero_rows.binary_search(&row).is_ok()
    }

    /// The role of cell `(row, col)`.
    #[must_use]
    pub fn role(&self, row: usize, col: usize) -> CellRole {
        if let Some(p) = self.pivots.iter().find(|p| p.row == row && p.col == col) {
            CellRole::Pivot { is_one: p.is_one }
        } else if self.to_clear.binary_search(&(row, col)).is_ok() {
            CellRole::ToClear
        } else {
            CellRole::Plain
        }
    }

    /// Short German phrases describing what is still missing.
    ///
    /// The list is empty exactly when [`Analysis::is_rref`] holds. With
    /// `compact` the phrases are shortened for narrow terminals.
    #[must_use]
    pub fn missing(&self, compact: bool) -> Vec<String> {
        let normalise = self
            .issues
            .iter()
            .filter(|i| matches!(i, Issue::PivotNotOne { .. }))
            .count();
        let clear = self.to_clear.len();
        let shape = self.issues.iter().any(|i| {
            matches!(
                i,
                Issue::ZeroRowAboveNonzero { .. } | Issue::PivotOrder { .. }
            )
        });
        let mut parts = Vec::new();
        if shape {
            parts.push("Stufenform fehlt".to_owned());
        }
        match normalise {
            0 => {}
            1 => parts.push("1 Pivot ≠ 1".to_owned()),
            n => parts.push(format!("{n} Pivots ≠ 1")),
        }
        let (one, many) = if compact {
            ("1 Eintrag ≠ 0", "Einträge ≠ 0")
        } else {
            (
                "1 Eintrag ≠ 0 in Pivotspalte",
                "Einträge ≠ 0 in Pivotspalten",
            )
        };
        match clear {
            0 => {}
            1 => parts.push(one.to_owned()),
            n => parts.push(format!("{n} {many}")),
        }
        parts
    }

    /// Number of rows of the analysed matrix.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns of the analysed matrix.
    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }
}

/// Why the reference solver performs a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Bring a nonzero entry into the pivot position by swapping rows.
    MovePivot,
    /// Scale the pivot row so that the pivot becomes 1.
    Normalise,
    /// Clear another entry of the pivot column.
    Eliminate,
}

/// One step of the reference solution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The row operation to perform.
    pub op: RowOperation,
    /// Zero-based column of the pivot being worked on.
    pub column: usize,
    /// Zero-based row in which that pivot is to stand.
    pub pivot_row: usize,
    /// Why this step is needed.
    pub purpose: Purpose,
}

impl Step {
    /// Whether the step is of the given operation kind.
    #[must_use]
    pub fn kind(&self) -> OpKind {
        self.op.kind()
    }
}

/// The reference solution of a matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solution {
    /// The steps, in order.
    pub steps: Vec<Step>,
    /// The Treppennormalform reached after all steps.
    pub result: Matrix,
}

/// Reference solver: deterministic Gauss–Jordan elimination.
///
/// Columns are processed from left to right while `r` counts the pivots
/// found so far. For column `c`:
///
/// 1. **Choose the pivot row** among rows `r..m` with a nonzero entry in `c`.
///    Tie-breaking rule: take the topmost row whose entry is exactly 1
///    (no scaling and no fractions needed); if there is none, take the
///    topmost nonzero entry. Columns without a candidate are skipped.
/// 2. If the chosen row is not row `r`, **swap** it into place.
/// 3. If the pivot is not 1, **scale** row `r` by its reciprocal.
/// 4. For every other row `i` (top to bottom, above *and* below) with a
///    nonzero entry in `c`, **add** the `(−a_ic)`-fold of row `r` to row `i`.
///
/// Steps that would change nothing are never emitted, so running the solver
/// on a partially reduced matrix continues where the student stopped.
///
/// Complexity: at most `min(m, n)` pivots with *O(m)* steps each, every step
/// costing *O(n)* rational operations, i.e. *O(m²·n)* in total.
#[must_use]
pub fn solve(m: &Matrix) -> Solution {
    let mut a = m.clone();
    let mut steps = Vec::new();
    let mut r = 0;
    for c in 0..a.cols() {
        if r == a.rows() {
            break;
        }
        let candidates = (r..a.rows()).filter(|&i| a.get(i, c).is_some_and(|q| !q.is_zero()));
        let pivot_row = candidates
            .clone()
            .find(|&i| a.get(i, c).is_some_and(|q| q.is_one()))
            .or_else(|| candidates.clone().next());
        let Some(p) = pivot_row else { continue };

        if p != r {
            let op = RowOperation::Swap {
                first: r,
                second: p,
            };
            a.swap_rows(r, p);
            steps.push(Step {
                op,
                column: c,
                pivot_row: r,
                purpose: Purpose::MovePivot,
            });
        }
        let pivot = a.get(r, c).cloned().unwrap_or_else(Q::one);
        if let Some(inv) = rational::inverse(&pivot).filter(|_| !pivot.is_one()) {
            a.scale_row(r, &inv);
            let op = RowOperation::Scale {
                row: r,
                factor: inv,
            };
            steps.push(Step {
                op,
                column: c,
                pivot_row: r,
                purpose: Purpose::Normalise,
            });
        }
        for i in 0..a.rows() {
            if i == r {
                continue;
            }
            let entry = a.get(i, c).cloned().unwrap_or_else(Q::zero);
            if !entry.is_zero() {
                let factor = -entry;
                a.add_multiple(&factor, r, i);
                let op = RowOperation::AddMultiple {
                    factor,
                    source: r,
                    target: i,
                };
                steps.push(Step {
                    op,
                    column: c,
                    pivot_row: r,
                    purpose: Purpose::Eliminate,
                });
            }
        }
        r += 1;
    }
    Solution { steps, result: a }
}

/// The unique Treppennormalform of `m`.
#[must_use]
pub fn rref(m: &Matrix) -> Matrix {
    solve(m).result
}

/// The rank of `m`: the number of nonzero rows of its Treppennormalform.
#[must_use]
pub fn rank(m: &Matrix) -> usize {
    let r = rref(m);
    (0..r.rows()).filter(|&i| !r.row_is_zero(i)).count()
}

/// Whether `m` is in Treppennormalform.
#[must_use]
pub fn is_rref(m: &Matrix) -> bool {
    Analysis::new(m).is_rref()
}

/// The first step of the reference solution for the *current* state of `m`,
/// or `None` if `m` is already in Treppennormalform.
#[must_use]
pub fn next_step(m: &Matrix) -> Option<Step> {
    solve(m).steps.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::{int, ratio};

    fn m(rows: &[&[i64]]) -> Matrix {
        Matrix::from_ints(rows).unwrap()
    }

    #[test]
    fn solves_the_course_example() {
        let a = m(&[&[1, -2, 3, -1], &[2, -1, 2, 2], &[3, 1, 2, 3]]);
        let expected = Matrix::from_rows(vec![
            vec![int(1), int(0), int(0), ratio(15, 7).unwrap()],
            vec![int(0), int(1), int(0), ratio(-4, 7).unwrap()],
            vec![int(0), int(0), int(1), ratio(-10, 7).unwrap()],
        ])
        .unwrap();
        let sol = solve(&a);
        assert_eq!(sol.result, expected);
        assert_eq!(rank(&a), 3);
        // Replaying the steps from A₀ reproduces the result.
        let mut replay = a;
        for s in &sol.steps {
            s.op.apply(&mut replay).unwrap();
        }
        assert_eq!(replay, expected);
    }

    #[test]
    fn solver_prefers_a_pivot_that_is_already_one() {
        let a = m(&[&[2, 1], &[1, 1]]);
        let first = next_step(&a).unwrap();
        assert_eq!(
            first.op,
            RowOperation::Swap {
                first: 0,
                second: 1
            }
        );
        assert_eq!(first.purpose, Purpose::MovePivot);
        assert_eq!((first.column, first.pivot_row), (0, 0));
    }

    #[test]
    fn solver_swaps_when_the_corner_is_zero() {
        let a = m(&[&[0, 1], &[3, 4]]);
        let first = next_step(&a).unwrap();
        assert_eq!(
            first.op,
            RowOperation::Swap {
                first: 0,
                second: 1
            }
        );
    }

    #[test]
    fn solver_is_idempotent_on_normal_forms() {
        let a = m(&[&[1, 0, 5], &[0, 1, -2], &[0, 0, 0]]);
        assert!(solve(&a).steps.is_empty());
        assert!(next_step(&a).is_none());
        assert_eq!(rref(&a), a);
    }

    #[test]
    fn handles_zero_and_dependent_matrices() {
        let z = m(&[&[0, 0], &[0, 0]]);
        assert_eq!(rank(&z), 0);
        assert!(is_rref(&z));
        let d = m(&[&[1, 2], &[2, 4], &[3, 6]]);
        assert_eq!(rank(&d), 1);
        assert_eq!(rref(&d), m(&[&[1, 2], &[0, 0], &[0, 0]]));
    }

    #[test]
    fn classifies_cells() {
        let a = m(&[&[2, 3, 0], &[0, 1, 4], &[0, 0, 0]]);
        let an = Analysis::new(&a);
        assert_eq!(an.role(0, 0), CellRole::Pivot { is_one: false });
        assert_eq!(an.role(1, 1), CellRole::Pivot { is_one: true });
        assert_eq!(an.role(0, 1), CellRole::ToClear);
        assert_eq!(an.role(1, 2), CellRole::Plain);
        assert!(an.is_pivot_col(0) && an.is_pivot_col(1) && !an.is_pivot_col(2));
        assert!(an.is_zero_row(2) && !an.is_zero_row(0));
        assert_eq!(an.rank(), 2);
        assert!(!an.is_rref());
        assert_eq!(
            an.missing(false),
            vec!["1 Pivot ≠ 1", "1 Eintrag ≠ 0 in Pivotspalte"]
        );
        assert_eq!(an.missing(true), vec!["1 Pivot ≠ 1", "1 Eintrag ≠ 0"]);
    }

    #[test]
    fn detects_structural_problems() {
        // zero row on top, pivots out of order
        let a = m(&[&[0, 0], &[0, 1], &[1, 0]]);
        let an = Analysis::new(&a);
        assert!(an.issues().contains(&Issue::ZeroRowAboveNonzero { row: 0 }));
        assert!(an.issues().contains(&Issue::PivotOrder { row: 2 }));
        assert_eq!(an.missing(true)[0], "Stufenform fehlt");
    }

    #[test]
    fn only_the_topmost_entry_of_a_column_is_its_pivot() {
        // Rows 2 and 3 start in column 1 too: those entries must become 0, they are no pivots.
        let a = Analysis::new(&Matrix::course_example());
        assert_eq!(
            a.pivots(),
            &[Pivot {
                row: 0,
                col: 0,
                is_one: true
            }]
        );
        assert_eq!(a.to_clear(), &[(1, 0), (2, 0)]);
        assert_eq!(a.role(1, 0), CellRole::ToClear);
        assert_eq!(a.missing(true), vec!["2 Einträge ≠ 0"]);
        assert_eq!(a.rank(), 3);

        // After clearing column 1, row 3 is blocked by the pivot of row 2 in column 2.
        let b = Analysis::new(&m(&[&[1, -2, 3, -1], &[0, 3, -4, 4], &[0, 7, -7, 6]]));
        assert_eq!(b.pivots().len(), 2);
        assert_eq!(b.role(1, 1), CellRole::Pivot { is_one: false });
        assert_eq!(b.role(2, 1), CellRole::ToClear);
        assert_eq!(b.role(0, 1), CellRole::ToClear);
    }

    #[test]
    fn shared_pivot_column_must_be_cleared() {
        let a = m(&[&[1, 0], &[1, 0]]);
        let an = Analysis::new(&a);
        assert_eq!(an.to_clear(), &[(1, 0)]);
        assert!(!an.is_rref());
    }

    #[test]
    fn accepts_the_identity_and_wide_forms() {
        assert!(is_rref(&m(&[&[1, 0], &[0, 1]])));
        assert!(is_rref(&m(&[&[1, 2, 0, 3], &[0, 0, 1, 4]])));
        assert!(!is_rref(&m(&[&[1, 2, 3], &[0, 0, 1]])));
        assert!(Analysis::new(&m(&[&[1, 0]])).missing(false).is_empty());
    }
}
