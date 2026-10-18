//! Reproducible generation of practice matrices.
//!
//! The generator uses its own tiny PRNG (SplitMix64) instead of an external
//! crate: the sequence for a given `--seed` is then fixed forever, independent
//! of dependency upgrades.

use crate::echelon::{Analysis, rank};
use crate::matrix::{MAX_DIM, Matrix, MatrixError};
use crate::rational::{Q, int};
use num_traits::Zero;

/// Smallest entry produced by the generator.
pub const ENTRY_MIN: i64 = -5;
/// Largest entry produced by the generator.
pub const ENTRY_MAX: i64 = 5;
/// Default number of rows.
pub const DEFAULT_ROWS: usize = 3;
/// Default number of columns.
pub const DEFAULT_COLS: usize = 4;

/// SplitMix64: a small, fast, well-distributed 64-bit PRNG.
///
/// Not cryptographically secure; it only drives exercise generation.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Creates a generator from `seed`. Equal seeds give equal sequences.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform number in `0..n` (returns 0 for `n == 0`), free of modulo bias.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let threshold = n.wrapping_neg() % n;
        loop {
            let x = self.next_u64();
            if x >= threshold {
                return x % n;
            }
        }
    }

    /// A uniform number in `lo..=hi`. Returns `lo` if `hi < lo`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi < lo {
            return lo;
        }
        let span = hi.wrapping_sub(lo) as u64;
        match span.checked_add(1) {
            Some(n) => lo.wrapping_add(self.below(n) as i64),
            None => self.next_u64() as i64,
        }
    }

    /// A uniform index in `0..len` (0 for an empty range).
    pub fn index(&mut self, len: usize) -> usize {
        self.below(len as u64) as usize
    }
}

/// The kind of exercise to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preset {
    /// Random entries without further constraints.
    #[default]
    Generic,
    /// `a₁₁ = 0`, so a row swap is needed right away.
    ZeroTopLeft,
    /// Contains a linearly dependent row; a zero row appears.
    DependentRow,
    /// More columns than rows.
    MoreColumns,
    /// More rows than columns.
    MoreRows,
}

impl Preset {
    /// All presets in menu order.
    pub const ALL: [Self; 5] = [
        Self::Generic,
        Self::ZeroTopLeft,
        Self::DependentRow,
        Self::MoreColumns,
        Self::MoreRows,
    ];

    /// German display name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Generic => "allgemein",
            Self::ZeroTopLeft => "a₁₁ = 0",
            Self::DependentRow => "abhängige Zeile",
            Self::MoreColumns => "mehr Spalten als Zeilen",
            Self::MoreRows => "mehr Zeilen als Spalten",
        }
    }

    /// The keyword accepted by the `new` command and `--preset`.
    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Generic => "allgemein",
            Self::ZeroTopLeft => "a11",
            Self::DependentRow => "abhängig",
            Self::MoreColumns => "breit",
            Self::MoreRows => "hoch",
        }
    }

    /// Parses a keyword (case-insensitive; German and English aliases).
    #[must_use]
    pub fn from_keyword(word: &str) -> Option<Self> {
        match word.to_lowercase().as_str() {
            "allgemein" | "generic" | "zufall" | "random" => Some(Self::Generic),
            "a11" | "a₁₁" | "null" | "tausch" | "swap" => Some(Self::ZeroTopLeft),
            "abhängig" | "abhaengig" | "dependent" | "nullzeile" | "dep" => {
                Some(Self::DependentRow)
            }
            "breit" | "wide" | "spalten" => Some(Self::MoreColumns),
            "hoch" | "tall" | "zeilen" => Some(Self::MoreRows),
            _ => None,
        }
    }

    /// Adjusts requested dimensions so that the preset is satisfiable.
    ///
    /// Dimensions are first clamped to `1..=MAX_DIM`. `MoreColumns` then
    /// ensures `cols > rows` (lowering `rows` to 5 if necessary), `MoreRows`
    /// ensures `rows > cols`, and the presets that need a second row
    /// (`ZeroTopLeft`, `DependentRow`) ensure `rows >= 2`.
    #[must_use]
    pub fn fit_dims(self, rows: usize, cols: usize) -> (usize, usize) {
        let rows = rows.clamp(1, MAX_DIM);
        let cols = cols.clamp(1, MAX_DIM);
        match self {
            Self::Generic => (rows, cols),
            Self::ZeroTopLeft | Self::DependentRow => (rows.max(2), cols),
            Self::MoreColumns => {
                let rows = rows.min(MAX_DIM - 1);
                (rows, cols.max(rows + 1))
            }
            Self::MoreRows => {
                let cols = cols.min(MAX_DIM - 1);
                (rows.max(cols + 1), cols)
            }
        }
    }
}

const MAX_TRIES: usize = 64;

fn random_rows(rng: &mut Rng, rows: usize, cols: usize) -> Vec<Vec<Q>> {
    (0..rows)
        .map(|_| {
            (0..cols)
                .map(|_| int(rng.range(ENTRY_MIN, ENTRY_MAX)))
                .collect()
        })
        .collect()
}

fn is_interesting(m: &Matrix) -> bool {
    rank(m) > 0 && !Analysis::new(m).is_rref()
}

fn random_nonzero(rng: &mut Rng) -> i64 {
    let v = rng.range(1, ENTRY_MAX);
    if rng.below(2) == 0 { v } else { -v }
}

/// A small nonzero coefficient for linear combinations: one of ±1, ±2.
fn small_coefficient(rng: &mut Rng) -> Q {
    const CHOICES: [i64; 4] = [-2, -1, 1, 2];
    int(CHOICES[rng.index(CHOICES.len())])
}

/// Generates a matrix for `preset`.
///
/// The dimensions are adapted with [`Preset::fit_dims`]; the returned matrix
/// carries the final size. The result is never the zero matrix and never
/// already in Treppennormalform.
///
/// # Errors
///
/// Cannot fail for any input in practice (the dimensions are clamped), but
/// the matrix constructor's error is propagated rather than hidden.
pub fn generate(
    rng: &mut Rng,
    rows: usize,
    cols: usize,
    preset: Preset,
) -> Result<Matrix, MatrixError> {
    let (rows, cols) = preset.fit_dims(rows, cols);
    let mut last = Matrix::from_rows(random_rows(rng, rows, cols))?;
    for _ in 0..MAX_TRIES {
        let mut data = random_rows(rng, rows, cols);
        match preset {
            Preset::Generic | Preset::MoreColumns | Preset::MoreRows => {}
            Preset::ZeroTopLeft => {
                data[0][0] = int(0);
                let k = 1 + rng.index(rows - 1);
                data[k][0] = int(random_nonzero(rng));
            }
            Preset::DependentRow => make_dependent(rng, &mut data),
        }
        last = Matrix::from_rows(data)?;
        if is_interesting(&last) {
            return Ok(last);
        }
    }
    Ok(last)
}

/// Overwrites one row with an integer combination of one or two other rows.
fn make_dependent(rng: &mut Rng, data: &mut [Vec<Q>]) {
    let rows = data.len();
    let target = rng.index(rows);
    let others: Vec<usize> = (0..rows).filter(|&i| i != target).collect();
    let first = others[rng.index(others.len())];
    let mut combined: Vec<Q> = {
        let c = small_coefficient(rng);
        data[first].iter().map(|q| q * &c).collect()
    };
    if combined.iter().all(Zero::is_zero) {
        combined = data[first].clone();
    }
    if others.len() >= 2 && rng.below(2) == 0 {
        let rest: Vec<usize> = others.iter().copied().filter(|&i| i != first).collect();
        let second = rest[rng.index(rest.len())];
        let c = small_coefficient(rng);
        for (acc, q) in combined.iter_mut().zip(&data[second]) {
            *acc += q * &c;
        }
    }
    data[target] = combined;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::echelon::next_step;
    use crate::ops::OpKind;

    #[test]
    fn rng_is_reproducible_and_spreads() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let xs: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys);
        let mut c = Rng::new(43);
        assert_ne!(xs[0], c.next_u64());
    }

    #[test]
    fn splitmix_reference_values() {
        // First outputs of SplitMix64 with seed 0 (published reference values).
        let mut r = Rng::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn range_covers_both_ends_and_respects_bounds() {
        let mut r = Rng::new(1);
        let mut seen = [false; 11];
        for _ in 0..2_000 {
            let v = r.range(-5, 5);
            assert!((-5..=5).contains(&v));
            seen[(v + 5) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(r.range(3, 3), 3);
        assert_eq!(r.range(5, 1), 5);
        assert_eq!(r.below(0), 0);
    }

    #[test]
    fn fits_dimensions_to_presets() {
        assert_eq!(Preset::Generic.fit_dims(0, 9), (1, 6));
        assert_eq!(Preset::MoreColumns.fit_dims(4, 2), (4, 5));
        assert_eq!(Preset::MoreColumns.fit_dims(6, 6), (5, 6));
        assert_eq!(Preset::MoreRows.fit_dims(2, 4), (5, 4));
        assert_eq!(Preset::MoreRows.fit_dims(3, 6), (6, 5));
        assert_eq!(Preset::ZeroTopLeft.fit_dims(1, 3), (2, 3));
        assert_eq!(Preset::DependentRow.fit_dims(1, 1), (2, 1));
    }

    #[test]
    fn presets_deliver_what_they_promise() {
        for seed in 0..60 {
            let mut rng = Rng::new(seed);
            for preset in Preset::ALL {
                let m = generate(&mut rng, DEFAULT_ROWS, DEFAULT_COLS, preset).unwrap();
                assert!(is_interesting(&m), "{preset:?} seed {seed}: {m}");
                match preset {
                    Preset::Generic => {}
                    Preset::ZeroTopLeft => {
                        assert_eq!(m[(0, 0)], int(0));
                        assert_eq!(next_step(&m).unwrap().kind(), OpKind::Swap);
                    }
                    Preset::DependentRow => assert!(rank(&m) < m.rows(), "{m}"),
                    Preset::MoreColumns => assert!(m.cols() > m.rows()),
                    Preset::MoreRows => assert!(m.rows() > m.cols()),
                }
            }
        }
    }

    #[test]
    fn same_seed_same_matrix() {
        let a = generate(&mut Rng::new(7), 3, 4, Preset::Generic).unwrap();
        let b = generate(&mut Rng::new(7), 3, 4, Preset::Generic).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn handles_the_smallest_sizes() {
        let mut rng = Rng::new(5);
        for preset in Preset::ALL {
            let m = generate(&mut rng, 1, 1, preset).unwrap();
            assert!(m.rows() >= 1 && m.cols() >= 1);
        }
    }

    #[test]
    fn parses_preset_keywords() {
        for p in Preset::ALL {
            assert_eq!(Preset::from_keyword(p.keyword()), Some(p));
        }
        assert_eq!(Preset::from_keyword("WIDE"), Some(Preset::MoreColumns));
        assert_eq!(Preset::from_keyword("quark"), None);
    }
}
