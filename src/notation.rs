//! Typographic helpers for the paper notation A₀ → A₁ → A₂ …

/// The true minus sign (U+2212) used when rendering matrix entries.
pub const MINUS: char = '−';

/// Converts a number to Unicode subscript digits, e.g. `12` → `₁₂`.
#[must_use]
pub fn subscript(n: usize) -> String {
    n.to_string()
        .chars()
        .map(|c| match c {
            '0' => '₀',
            '1' => '₁',
            '2' => '₂',
            '3' => '₃',
            '4' => '₄',
            '5' => '₅',
            '6' => '₆',
            '7' => '₇',
            '8' => '₈',
            _ => '₉',
        })
        .collect()
}

/// Names the matrix after `step` operations: `A₀` is the initial matrix.
#[must_use]
pub fn matrix_label(step: usize) -> String {
    format!("A{}", subscript(step))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscripts_digits() {
        assert_eq!(subscript(0), "₀");
        assert_eq!(subscript(12), "₁₂");
        assert_eq!(subscript(907), "₉₀₇");
    }

    #[test]
    fn labels_matrices() {
        assert_eq!(matrix_label(0), "A₀");
        assert_eq!(matrix_label(3), "A₃");
    }
}
