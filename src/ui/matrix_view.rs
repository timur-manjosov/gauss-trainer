//! Typeset rendering of matrices.
//!
//! Entries are drawn as stacked fractions (numerator, bar, denominator) with
//! a true minus sign (−) in front of each negative entry, right-aligned
//! within its column. Rows are enclosed by tall Unicode brackets. The matrix is
//! first drawn onto a grid of styled characters and then converted to
//! ratatui lines.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::echelon::{Analysis, CellRole};
use crate::matrix::Matrix;
use crate::notation::MINUS;
use crate::rational::{Parts, parts};

use super::palette::Palette;

/// Blank space around and between the cells of a matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spacing {
    /// Blank lines between two rows.
    pub rows: usize,
    /// Blank characters between two columns.
    pub cols: usize,
}

impl Spacing {
    /// Comfortable spacing: three characters between columns.
    pub const fn new(rows: usize) -> Self {
        Self { rows, cols: 3 }
    }
}

/// Width of the annotation shown next to zero rows.
const ANNOTATION: &str = "Nullzeile";

/// How fractional entries are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStyle {
    /// Numerator over bar over denominator (three lines per row).
    Stacked,
    /// `a/b` on a single line.
    Inline,
}

/// Which characters form the brackets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Brackets {
    /// `⎡ ⎢ ⎣` and `⎤ ⎥ ⎦` (Miscellaneous Technical block).
    #[default]
    Tall,
    /// Box-drawing corners `┌ │ └`, available in every terminal font.
    Simple,
}

impl Brackets {
    fn left(self, y: usize, height: usize) -> char {
        match (self, height, y) {
            (_, 1, _) => '[',
            (Self::Tall, _, 0) => '⎡',
            (Self::Tall, _, y) if y + 1 == height => '⎣',
            (Self::Tall, _, _) => '⎢',
            (Self::Simple, _, 0) => '┌',
            (Self::Simple, _, y) if y + 1 == height => '└',
            (Self::Simple, _, _) => '│',
        }
    }

    fn right(self, y: usize, height: usize) -> char {
        match (self, height, y) {
            (_, 1, _) => ']',
            (Self::Tall, _, 0) => '⎤',
            (Self::Tall, _, y) if y + 1 == height => '⎦',
            (Self::Tall, _, _) => '⎥',
            (Self::Simple, _, 0) => '┐',
            (Self::Simple, _, y) if y + 1 == height => '┘',
            (Self::Simple, _, _) => '│',
        }
    }
}

/// Extra information drawn on top of the plain matrix.
#[derive(Debug, Clone, Copy, Default)]
pub struct Decor<'a> {
    /// Live analysis; `None` draws a neutral matrix.
    pub analysis: Option<&'a Analysis>,
    /// Zero-based rows to tint as "changed by the last step".
    pub changed: &'a [usize],
    /// Whether to draw row and column numbers.
    pub labels: bool,
}

/// One column: wide enough for its widest entry including the minus sign.
#[derive(Debug, Clone, Copy)]
struct Column {
    width: usize,
}

/// Measurements of a typeset matrix.
#[derive(Debug, Clone)]
pub struct Geometry {
    entry_style: EntryStyle,
    row_h: usize,
    spacing: Spacing,
    labels: bool,
    annotate: bool,
    columns: Vec<Column>,
    /// Total width in characters.
    pub width: usize,
    /// Total height in lines.
    pub height: usize,
}

/// Width of the digits (or of the fraction with its overhanging bar), without the sign.
fn body_width(style: EntryStyle, p: &Parts) -> usize {
    match (&p.denom, style) {
        (None, _) => p.numer.chars().count(),
        (Some(d), EntryStyle::Stacked) => p.numer.chars().count().max(d.chars().count()) + 2,
        (Some(d), EntryStyle::Inline) => p.numer.chars().count() + 1 + d.chars().count(),
    }
}

/// Width of the minus sign including the space that keeps it apart from a fraction bar.
fn sign_width(style: EntryStyle, p: &Parts) -> usize {
    match (p.negative, &p.denom, style) {
        (false, _, _) => 0,
        (true, Some(_), EntryStyle::Stacked) => 2,
        (true, _, _) => 1,
    }
}

/// Total width of an entry including its sign.
fn entry_width(style: EntryStyle, p: &Parts) -> usize {
    sign_width(style, p) + body_width(style, p)
}

impl Geometry {
    /// Measures `matrix` drawn with `entry_style` and `spacing`. `labels`
    /// adds row/column numbers, `annotate` reserves room for the zero-row
    /// annotation.
    #[must_use]
    pub fn new(
        matrix: &Matrix,
        entry_style: EntryStyle,
        spacing: Spacing,
        labels: bool,
        annotate: bool,
    ) -> Self {
        let all: Vec<Vec<Parts>> = matrix
            .iter_rows()
            .map(|r| r.iter().map(parts).collect())
            .collect();
        let has_fraction = all.iter().flatten().any(|p| p.denom.is_some());
        let row_h = if has_fraction && entry_style == EntryStyle::Stacked {
            3
        } else {
            1
        };
        let columns: Vec<Column> = (0..matrix.cols())
            .map(|j| Column {
                width: all
                    .iter()
                    .filter_map(|row| row.get(j))
                    .map(|p| entry_width(entry_style, p))
                    .max()
                    .unwrap_or(1),
            })
            .collect();
        let cells: usize = columns.iter().map(|c| c.width).sum::<usize>()
            + spacing.cols * columns.len().saturating_sub(1);
        let prefix = if labels { 3 } else { 0 };
        // prefix, bracket, pad, cells, pad, bracket
        let mut width = prefix + 2 + cells + 2;
        if annotate {
            width += 2 + ANNOTATION.chars().count();
        }
        let body_h = matrix.rows() * row_h + matrix.rows().saturating_sub(1) * spacing.rows;
        Self {
            entry_style,
            row_h,
            spacing,
            labels,
            annotate,
            columns,
            width,
            height: body_h + usize::from(labels),
        }
    }

    fn prefix(&self) -> usize {
        if self.labels { 3 } else { 0 }
    }

    fn header_h(&self) -> usize {
        usize::from(self.labels)
    }

    fn body_h(&self) -> usize {
        self.height - self.header_h()
    }

    fn row_top(&self, row: usize) -> usize {
        self.header_h() + row * (self.row_h + self.spacing.rows)
    }

    fn column_x(&self, col: usize) -> usize {
        let before: usize = self
            .columns
            .iter()
            .take(col)
            .map(|c| c.width + self.spacing.cols)
            .sum();
        self.prefix() + 2 + before
    }

    fn cells_end(&self) -> usize {
        let last = self.columns.len().saturating_sub(1);
        self.columns
            .get(last)
            .map_or(self.prefix() + 2, |c| self.column_x(last) + c.width)
    }
}

/// A grid of styled characters.
struct Canvas {
    cells: Vec<Vec<(char, Style)>>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![(' ', Style::default()); width]; height],
        }
    }

    fn put(&mut self, x: usize, y: usize, text: &str, style: Style) {
        let Some(row) = self.cells.get_mut(y) else {
            return;
        };
        for (i, ch) in text.chars().enumerate() {
            if let Some(cell) = row.get_mut(x + i) {
                *cell = (ch, cell.1.patch(style));
            }
        }
    }

    fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, style: Style) {
        for row in self.cells.iter_mut().skip(y).take(h) {
            for cell in row.iter_mut().skip(x).take(w) {
                cell.1 = cell.1.patch(style);
            }
        }
    }

    fn into_lines(self, indent: usize) -> Vec<Line<'static>> {
        self.cells
            .into_iter()
            .map(|row| {
                let mut spans = Vec::new();
                if indent > 0 {
                    spans.push(Span::raw(" ".repeat(indent)));
                }
                let mut text = String::new();
                let mut current: Option<Style> = None;
                for (ch, style) in row {
                    if let Some(c) = current.filter(|c| *c != style) {
                        spans.push(Span::styled(std::mem::take(&mut text), c));
                    }
                    current = Some(style);
                    text.push(ch);
                }
                if let Some(c) = current {
                    spans.push(Span::styled(text, c));
                }
                Line::from(spans)
            })
            .collect()
    }
}

fn cell_style(pal: &Palette, decor: &Decor<'_>, row: usize, col: usize, is_zero: bool) -> Style {
    let mut style = Style::default().fg(if is_zero { pal.dim } else { pal.fg });
    if let Some(analysis) = decor.analysis {
        match analysis.role(row, col) {
            CellRole::Pivot { is_one } => {
                let (fg, bg) = if is_one {
                    (pal.green, pal.pivot_one_bg)
                } else {
                    (pal.amber, pal.pivot_other_bg)
                };
                style = style.fg(fg).bg(bg).add_modifier(Modifier::BOLD);
            }
            CellRole::ToClear => {
                style = style
                    .fg(pal.red)
                    .bg(pal.clear_bg)
                    .add_modifier(Modifier::BOLD);
            }
            CellRole::Plain => {}
        }
    }
    style
}

/// Draws `matrix` into styled lines (see the module documentation).
///
/// `indent` blanks are prepended to every line, which is used to centre the
/// matrix in its panel. The result has `geometry.height` lines.
#[must_use]
pub fn draw(
    matrix: &Matrix,
    geometry: &Geometry,
    decor: &Decor<'_>,
    pal: &Palette,
    brackets: Brackets,
    indent: usize,
) -> Vec<Line<'static>> {
    let mut canvas = Canvas::new(geometry.width, geometry.height);
    let body_top = geometry.header_h();
    let body_h = geometry.body_h();
    let cells_start = geometry.prefix() + 2;
    let cells_end = geometry.cells_end();

    // Backgrounds: pivot-column bands first, changed-row tints on top.
    if let Some(a) = decor.analysis {
        for (j, col) in geometry.columns.iter().enumerate() {
            if a.is_pivot_col(j) {
                canvas.fill(
                    geometry.column_x(j),
                    body_top,
                    col.width,
                    body_h,
                    Style::default().bg(pal.pivot_col_bg),
                );
            }
        }
    }
    for &row in decor.changed {
        if row < matrix.rows() {
            let top = geometry.row_top(row);
            canvas.fill(
                cells_start,
                top,
                cells_end - cells_start,
                geometry.row_h,
                Style::default().bg(pal.changed_bg),
            );
        }
    }

    // Brackets.
    let bracket_style = Style::default().fg(pal.accent2);
    for y in 0..body_h {
        canvas.put(
            geometry.prefix(),
            body_top + y,
            &brackets.left(y, body_h).to_string(),
            bracket_style,
        );
        canvas.put(
            cells_end + 1,
            body_top + y,
            &brackets.right(y, body_h).to_string(),
            bracket_style,
        );
    }

    // Header with column numbers.
    if geometry.labels {
        for (j, col) in geometry.columns.iter().enumerate() {
            let number = (j + 1).to_string();
            let pivot = decor.analysis.is_some_and(|a| a.is_pivot_col(j));
            let style = if pivot {
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(pal.dim)
            };
            let x = geometry.column_x(j) + col.width - number.chars().count();
            canvas.put(x, 0, &number, style);
        }
    }

    for (i, row) in matrix.iter_rows().enumerate() {
        let top = geometry.row_top(i);
        let mid = top + geometry.row_h / 2;

        if geometry.labels {
            if decor.changed.contains(&i) {
                for y in top..top + geometry.row_h {
                    canvas.put(0, y, "▎", Style::default().fg(pal.accent));
                }
            }
            canvas.put(1, mid, &(i + 1).to_string(), Style::default().fg(pal.dim));
        }
        if geometry.annotate && decor.analysis.is_some_and(|a| a.is_zero_row(i)) {
            let style = Style::default().fg(pal.dim).add_modifier(Modifier::ITALIC);
            canvas.put(cells_end + 3, mid, ANNOTATION, style);
        }

        for (j, q) in row.iter().enumerate() {
            let Some(col) = geometry.columns.get(j) else {
                continue;
            };
            let p = parts(q);
            let style = cell_style(pal, decor, i, j, num_traits::Zero::is_zero(q));
            let x0 = geometry.column_x(j);
            // Pivot and to-clear cells get their own background box.
            if let Some(bg) = style.bg {
                canvas.fill(x0, top, col.width, geometry.row_h, Style::default().bg(bg));
            }
            // Entries are right-aligned; the sign sits directly in front of the digits or bar.
            let start = x0 + col.width - entry_width(geometry.entry_style, &p);
            if p.negative {
                canvas.put(start, mid, &MINUS.to_string(), style);
            }
            let body_x = start + sign_width(geometry.entry_style, &p);
            match (&p.denom, geometry.entry_style) {
                (None, _) => canvas.put(body_x, mid, &p.numer, style),
                (Some(d), EntryStyle::Inline) => {
                    canvas.put(body_x, mid, &format!("{}/{d}", p.numer), style)
                }
                (Some(d), EntryStyle::Stacked) => {
                    let w = body_width(EntryStyle::Stacked, &p);
                    canvas.put(body_x, mid, &"─".repeat(w), style);
                    let nw = p.numer.chars().count();
                    let dw = d.chars().count();
                    canvas.put(body_x + (w - nw) / 2, mid - 1, &p.numer, style);
                    canvas.put(body_x + (w - dw) / 2, mid + 1, d, style);
                }
            }
        }
    }
    canvas.into_lines(indent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Theme;
    use crate::parse::parse_matrix;
    use crate::ui::palette::ColorDepth;

    fn plain(m: &Matrix, style: EntryStyle, gap: usize) -> Vec<String> {
        let pal = Palette::new(Theme::Dark, ColorDepth::TrueColor);
        let g = Geometry::new(m, style, Spacing::new(gap), false, false);
        draw(m, &g, &Decor::default(), &pal, Brackets::Tall, 0)
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn integer_matrices_use_one_line_per_row() {
        let m = parse_matrix("1 -2; 30 4").unwrap();
        let lines = plain(&m, EntryStyle::Stacked, 0);
        assert_eq!(lines, vec!["⎡  1   −2 ⎤", "⎣ 30    4 ⎦"]);
    }

    #[test]
    fn single_row_uses_plain_brackets() {
        let m = parse_matrix("1 2 3").unwrap();
        assert_eq!(plain(&m, EntryStyle::Stacked, 0), vec!["[ 1   2   3 ]"]);
    }

    #[test]
    fn fractions_are_stacked_with_a_bar() {
        let m = parse_matrix("1 -3/4; 0 10/7").unwrap();
        let lines = plain(&m, EntryStyle::Stacked, 0);
        assert_eq!(
            lines,
            vec![
                "⎡        3  ⎤",
                "⎢ 1   − ─── ⎥",
                "⎢        4  ⎥",
                "⎢       10  ⎥",
                "⎢ 0    ──── ⎥",
                "⎣       7   ⎦",
            ]
        );
    }

    #[test]
    fn rows_can_be_separated_by_a_gap() {
        let m = parse_matrix("1; 2").unwrap();
        assert_eq!(
            plain(&m, EntryStyle::Stacked, 1),
            vec!["⎡ 1 ⎤", "⎢   ⎥", "⎣ 2 ⎦"]
        );
    }

    #[test]
    fn inline_fractions_stay_on_one_line() {
        let m = parse_matrix("1/2 -3; 4 5/6").unwrap();
        let lines = plain(&m, EntryStyle::Inline, 0);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("1/2") && lines[1].contains("5/6"));
    }

    #[test]
    fn uses_the_true_minus_sign_only() {
        let m = parse_matrix("-1 -1/2; 3 4").unwrap();
        for style in [EntryStyle::Stacked, EntryStyle::Inline] {
            let text = plain(&m, style, 0).join("\n");
            assert!(text.contains('−'));
            assert!(!text.contains('-'), "ASCII hyphen found:\n{text}");
        }
    }

    #[test]
    fn geometry_matches_the_drawn_size() {
        for src in [
            "1 2 3",
            "1/2 -3; 4 5/6",
            "1 0 0 15/7; 0 1 0 -4/7; 0 0 1 -10/7",
        ] {
            let m = parse_matrix(src).unwrap();
            for style in [EntryStyle::Stacked, EntryStyle::Inline] {
                for gap in 0..=1 {
                    let pal = Palette::new(Theme::Dark, ColorDepth::TrueColor);
                    let g = Geometry::new(&m, style, Spacing::new(gap), true, true);
                    let lines = draw(&m, &g, &Decor::default(), &pal, Brackets::Tall, 2);
                    assert_eq!(lines.len(), g.height);
                    assert!(lines.iter().all(|l| l.width() == g.width + 2), "{src}");
                }
            }
        }
    }

    #[test]
    fn analysis_marks_pivots_zero_rows_and_changes() {
        let m = parse_matrix("2 1; 0 0").unwrap();
        let a = Analysis::new(&m);
        let pal = Palette::new(Theme::Dark, ColorDepth::TrueColor);
        let g = Geometry::new(&m, EntryStyle::Stacked, Spacing::new(0), true, true);
        let decor = Decor {
            analysis: Some(&a),
            changed: &[1],
            labels: true,
        };
        let lines = draw(&m, &g, &decor, &pal, Brackets::Tall, 0);
        let text: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        assert!(text[2].contains("Nullzeile"), "{text:?}");
        assert!(text[2].starts_with('▎'));
        // The pivot "2" is amber (not yet 1) and bold.
        let pivot = lines[1]
            .spans
            .iter()
            .find(|s| s.content.contains('2'))
            .unwrap();
        assert_eq!(pivot.style.fg, Some(pal.amber));
        assert!(pivot.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn simple_brackets_avoid_the_technical_block() {
        let m = parse_matrix("1; 2; 3").unwrap();
        let pal = Palette::new(Theme::Dark, ColorDepth::TrueColor);
        let g = Geometry::new(&m, EntryStyle::Stacked, Spacing::new(0), false, false);
        let lines = draw(&m, &g, &Decor::default(), &pal, Brackets::Simple, 0);
        let first: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(first.starts_with('┌') && first.ends_with('┐'));
    }
}
