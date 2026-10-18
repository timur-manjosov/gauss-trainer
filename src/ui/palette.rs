//! Colour palettes for the dark and light theme.
//!
//! The palettes are defined in truecolor. On terminals without truecolor
//! support each colour is mapped to the nearest entry of the xterm 256-colour
//! palette, using integer arithmetic only.

use ratatui::style::Color;

use crate::app::Theme;

/// How many colours the terminal can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorDepth {
    /// 24-bit colour.
    #[default]
    TrueColor,
    /// The xterm 256-colour palette.
    Indexed256,
}

impl ColorDepth {
    /// Chooses a depth from the `COLORTERM` environment variable.
    ///
    /// `truecolor` and `24bit` select 24-bit colour; anything else falls
    /// back to 256 colours.
    #[must_use]
    pub fn from_colorterm(colorterm: Option<&str>) -> Self {
        match colorterm.map(str::to_ascii_lowercase).as_deref() {
            Some("truecolor" | "24bit") => Self::TrueColor,
            _ => Self::Indexed256,
        }
    }

    /// Converts an RGB triple to a terminal colour of this depth.
    #[must_use]
    pub fn color(self, r: u8, g: u8, b: u8) -> Color {
        match self {
            Self::TrueColor => Color::Rgb(r, g, b),
            Self::Indexed256 => Color::Indexed(nearest_indexed(r, g, b)),
        }
    }
}

/// The six channel levels of the xterm colour cube (indices 16–231).
const CUBE_LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];

fn nearest_level(v: u8) -> usize {
    let v = i32::from(v);
    let mut best = 0;
    for (i, &level) in CUBE_LEVELS.iter().enumerate() {
        if (v - level).abs() < (v - CUBE_LEVELS[best]).abs() {
            best = i;
        }
    }
    best
}

fn dist(a: (i32, i32, i32), b: (i32, i32, i32)) -> i32 {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2) + (a.2 - b.2).pow(2)
}

/// Maps an RGB colour to the nearest xterm-256 palette index (16–255).
///
/// Candidates are the best colour-cube entry and the best entry of the
/// 24-step grey ramp (indices 232–255); the one with the smaller squared
/// RGB distance wins.
#[must_use]
pub fn nearest_indexed(r: u8, g: u8, b: u8) -> u8 {
    let target = (i32::from(r), i32::from(g), i32::from(b));

    let (ri, gi, bi) = (nearest_level(r), nearest_level(g), nearest_level(b));
    let cube_rgb = (CUBE_LEVELS[ri], CUBE_LEVELS[gi], CUBE_LEVELS[bi]);
    let cube_index = 16 + 36 * ri + 6 * gi + bi;

    let avg = (target.0 + target.1 + target.2) / 3;
    let grey_step = ((avg - 8 + 5) / 10).clamp(0, 23);
    let grey_level = 8 + 10 * grey_step;
    let grey_index = 232 + grey_step;

    let index = if dist(target, (grey_level, grey_level, grey_level)) < dist(target, cube_rgb) {
        grey_index
    } else {
        i32::try_from(cube_index).unwrap_or(16)
    };
    u8::try_from(index).unwrap_or(16)
}

/// All colours used by the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Window background.
    pub bg: Color,
    /// Panel background.
    pub surface: Color,
    /// Background of the status line.
    pub bar: Color,
    /// Primary text.
    pub fg: Color,
    /// Secondary text.
    pub muted: Color,
    /// Disabled or de-emphasised text, zeros, hints.
    pub dim: Color,
    /// Panel borders.
    pub border: Color,
    /// Focus colour: titles, prompt, key names.
    pub accent: Color,
    /// Secondary accent: brackets, hints.
    pub accent2: Color,
    /// Success, pivots equal to 1.
    pub green: Color,
    /// Pivots that are not yet 1, warnings.
    pub amber: Color,
    /// Errors, entries that must become 0.
    pub red: Color,
    /// Background tint for pivot columns.
    pub pivot_col_bg: Color,
    /// Background tint for rows changed by the last step.
    pub changed_bg: Color,
    /// Background of a pivot that equals 1.
    pub pivot_one_bg: Color,
    /// Background of a pivot that is not 1.
    pub pivot_other_bg: Color,
    /// Background of entries that must become 0.
    pub clear_bg: Color,
}

impl Palette {
    /// Builds the palette of `theme` for a terminal of the given depth.
    #[must_use]
    pub fn new(theme: Theme, depth: ColorDepth) -> Self {
        let c = |r, g, b| depth.color(r, g, b);
        match theme {
            Theme::Dark => Self {
                bg: c(0x16, 0x17, 0x21),
                surface: c(0x1a, 0x1b, 0x26),
                bar: c(0x24, 0x28, 0x3b),
                fg: c(0xc0, 0xca, 0xf5),
                muted: c(0x9a, 0xa5, 0xce),
                dim: c(0x56, 0x5f, 0x89),
                border: c(0x3b, 0x42, 0x61),
                accent: c(0x7a, 0xa2, 0xf7),
                accent2: c(0xbb, 0x9a, 0xf7),
                green: c(0x9e, 0xce, 0x6a),
                amber: c(0xe0, 0xaf, 0x68),
                red: c(0xf7, 0x76, 0x8e),
                pivot_col_bg: c(0x22, 0x26, 0x3a),
                changed_bg: c(0x2b, 0x33, 0x55),
                pivot_one_bg: c(0x25, 0x38, 0x33),
                pivot_other_bg: c(0x3a, 0x33, 0x29),
                clear_bg: c(0x3b, 0x27, 0x35),
            },
            Theme::Light => Self {
                bg: c(0xe6, 0xe7, 0xed),
                surface: c(0xf1, 0xf2, 0xf7),
                bar: c(0xd8, 0xdb, 0xe8),
                fg: c(0x34, 0x3b, 0x58),
                muted: c(0x53, 0x5c, 0x85),
                dim: c(0x96, 0x9b, 0xb5),
                border: c(0xb4, 0xb9, 0xd0),
                accent: c(0x2e, 0x5f, 0xd3),
                accent2: c(0x7f, 0x3f, 0xd1),
                green: c(0x3d, 0x7a, 0x1f),
                amber: c(0x9a, 0x62, 0x00),
                red: c(0xc0, 0x2f, 0x4a),
                pivot_col_bg: c(0xe4, 0xe8, 0xf5),
                changed_bg: c(0xd3, 0xdd, 0xf6),
                pivot_one_bg: c(0xd7, 0xea, 0xcc),
                pivot_other_bg: c(0xf3, 0xe3, 0xc3),
                clear_bg: c(0xf5, 0xd9, 0xe0),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_colours_map_to_the_cube_corners() {
        assert_eq!(nearest_indexed(0, 0, 0), 16);
        assert_eq!(nearest_indexed(255, 255, 255), 231);
        assert_eq!(nearest_indexed(255, 0, 0), 196);
        assert_eq!(nearest_indexed(0, 255, 0), 46);
        assert_eq!(nearest_indexed(0, 0, 255), 21);
    }

    #[test]
    fn greys_prefer_the_grey_ramp() {
        assert_eq!(nearest_indexed(128, 128, 128), 244);
        assert_eq!(nearest_indexed(8, 8, 8), 232);
        assert_eq!(nearest_indexed(238, 238, 238), 255);
    }

    #[test]
    fn indices_stay_in_range() {
        for r in (0..=255u8).step_by(15) {
            for g in (0..=255u8).step_by(15) {
                for b in (0..=255u8).step_by(15) {
                    assert!(nearest_indexed(r, g, b) >= 16);
                }
            }
        }
    }

    #[test]
    fn detects_color_depth() {
        assert_eq!(
            ColorDepth::from_colorterm(Some("truecolor")),
            ColorDepth::TrueColor
        );
        assert_eq!(
            ColorDepth::from_colorterm(Some("24BIT")),
            ColorDepth::TrueColor
        );
        assert_eq!(
            ColorDepth::from_colorterm(Some("yes")),
            ColorDepth::Indexed256
        );
        assert_eq!(ColorDepth::from_colorterm(None), ColorDepth::Indexed256);
    }

    #[test]
    fn palettes_follow_the_depth() {
        let p = Palette::new(Theme::Dark, ColorDepth::TrueColor);
        assert!(matches!(p.accent, Color::Rgb(0x7a, 0xa2, 0xf7)));
        let p = Palette::new(Theme::Light, ColorDepth::Indexed256);
        assert!(matches!(p.accent, Color::Indexed(_)));
        assert_ne!(
            Palette::new(Theme::Dark, ColorDepth::TrueColor).bg,
            Palette::new(Theme::Light, ColorDepth::TrueColor).bg
        );
    }
}
