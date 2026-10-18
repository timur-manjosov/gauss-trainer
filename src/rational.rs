//! Exact rational numbers.
//!
//! Every scalar in this crate is a [`Q`], an arbitrary-precision fraction in
//! lowest terms with a positive denominator. There is no floating point
//! anywhere in the crate (`clippy::float_arithmetic` is denied), so results
//! are always exact.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

/// The scalar type of the crate: an exact rational number.
///
/// Invariant (maintained by `num-rational`): the denominator is positive and
/// coprime to the numerator.
pub type Q = BigRational;

/// Builds the rational number `n / 1`.
#[must_use]
pub fn int(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

/// Builds `numer / denom`, or returns `None` if `denom` is zero.
#[must_use]
pub fn ratio(numer: i64, denom: i64) -> Option<Q> {
    if denom == 0 {
        None
    } else {
        Some(Q::new(BigInt::from(numer), BigInt::from(denom)))
    }
}

/// Returns `true` if `q` equals one.
#[must_use]
pub fn is_one(q: &Q) -> bool {
    q.is_one()
}

/// Returns the multiplicative inverse of `q`, or `None` for zero.
#[must_use]
pub fn inverse(q: &Q) -> Option<Q> {
    if q.is_zero() { None } else { Some(q.recip()) }
}

/// A rational number split into the pieces a renderer needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parts {
    /// `true` for strictly negative numbers.
    pub negative: bool,
    /// Decimal digits of the absolute value of the numerator.
    pub numer: String,
    /// Decimal digits of the denominator, or `None` for integers.
    pub denom: Option<String>,
}

/// Splits `q` into sign, numerator and (for non-integers) denominator.
#[must_use]
pub fn parts(q: &Q) -> Parts {
    Parts {
        negative: q.is_negative(),
        numer: q.numer().abs().to_string(),
        denom: if q.denom().is_one() {
            None
        } else {
            Some(q.denom().to_string())
        },
    }
}

/// Formats `q` as plain text with an ASCII minus: `5`, `-3/4`.
///
/// Integers carry no denominator; this is the notation used by the
/// command line and the Rechenprotokoll.
#[must_use]
pub fn to_text(q: &Q) -> String {
    let p = parts(q);
    let sign = if p.negative { "-" } else { "" };
    match p.denom {
        None => format!("{sign}{}", p.numer),
        Some(d) => format!("{sign}{}/{d}", p.numer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_reduced_fractions() {
        assert_eq!(ratio(2, 4), ratio(1, 2));
        assert_eq!(ratio(3, -6), ratio(-1, 2));
        assert_eq!(ratio(1, 0), None);
    }

    #[test]
    fn formats_integers_and_fractions() {
        assert_eq!(to_text(&int(5)), "5");
        assert_eq!(to_text(&int(-7)), "-7");
        assert_eq!(to_text(&int(0)), "0");
        assert_eq!(to_text(&ratio(-3, 4).unwrap()), "-3/4");
        assert_eq!(to_text(&ratio(15, 7).unwrap()), "15/7");
    }

    #[test]
    fn splits_into_parts() {
        let p = parts(&ratio(-10, 7).unwrap());
        assert!(p.negative);
        assert_eq!(p.numer, "10");
        assert_eq!(p.denom.as_deref(), Some("7"));
        let p = parts(&int(4));
        assert!(!p.negative);
        assert_eq!(p.denom, None);
    }

    #[test]
    fn inverse_of_zero_is_none() {
        assert_eq!(inverse(&int(0)), None);
        assert_eq!(inverse(&int(-4)), ratio(-1, 4));
    }

    #[test]
    fn detects_one() {
        assert!(is_one(&ratio(5, 5).unwrap()));
        assert!(!is_one(&int(2)));
    }
}
