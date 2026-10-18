//! The Rechenprotokoll panel.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::app::App;
use crate::notation::matrix_label;

use super::palette::Palette;

/// Greedy word wrap to `width` characters; overlong words are split.
#[must_use]
pub fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut len = 0;
    for word in text.split_whitespace() {
        let mut word: &str = word;
        loop {
            let wlen = word.chars().count();
            let sep = usize::from(len > 0);
            if len + sep + wlen <= width {
                if sep == 1 {
                    current.push(' ');
                }
                current.push_str(word);
                len += sep + wlen;
                break;
            }
            if len > 0 {
                lines.push(std::mem::take(&mut current));
                len = 0;
                continue;
            }
            // The word alone is too long: split it.
            let split = word
                .char_indices()
                .nth(width)
                .map_or(word.len(), |(i, _)| i);
            let (head, tail) = word.split_at(split);
            lines.push(head.to_owned());
            word = tail;
            if word.is_empty() {
                break;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Descriptions narrower than this are put below the step label instead of beside it.
const MIN_SIDE_BY_SIDE: usize = 20;

/// Draws the protocol into `area` (the inside of the panel border).
pub fn render(frame: &mut Frame<'_>, area: Rect, app: &App, pal: &Palette) {
    let protocol = app.protocol();
    let width = usize::from(area.width);
    let label_w = protocol
        .iter()
        .map(|e| matrix_label(e.from).chars().count() + 3 + matrix_label(e.to).chars().count())
        .max()
        .unwrap_or(0)
        .max(matrix_label(0).chars().count());
    // marker, space, label, two spaces; two columns stay free for the scrollbar
    let side_prefix = 2 + label_w + 2;
    let side_by_side = width.saturating_sub(side_prefix + 2) >= MIN_SIDE_BY_SIDE;
    let prefix_w = if side_by_side { side_prefix } else { 4 };
    let text_w = width.saturating_sub(prefix_w + 2).max(8);

    let accent_bold = Style::default().fg(pal.accent).add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{:<label_w$}", matrix_label(0)), accent_bold),
        Span::raw("  "),
        Span::styled("Startmatrix", Style::default().fg(pal.muted)),
    ]));

    let mut last_done_end = lines.len();
    let mut undone_lines = 0;
    for entry in &protocol {
        let current = !entry.undone && entry.to == app.step();
        let (marker, marker_style) = if entry.undone {
            ("↷", Style::default().fg(pal.dim))
        } else if current {
            ("▸", accent_bold)
        } else {
            (" ", Style::default())
        };
        let label = format!("{} → {}", matrix_label(entry.from), matrix_label(entry.to));
        let (label_style, text_style) = if entry.undone {
            (
                Style::default().fg(pal.dim),
                Style::default().fg(pal.dim).add_modifier(Modifier::ITALIC),
            )
        } else if current {
            (accent_bold, Style::default().fg(pal.fg))
        } else {
            (Style::default().fg(pal.muted), Style::default().fg(pal.fg))
        };

        let mut entry_lines: Vec<Line<'static>> = Vec::new();
        let parts = wrap_words(&entry.op.describe(), text_w);
        if side_by_side {
            for (n, part) in parts.into_iter().enumerate() {
                entry_lines.push(Line::from(if n == 0 {
                    vec![
                        Span::styled(marker, marker_style),
                        Span::raw(" "),
                        Span::styled(format!("{label:<label_w$}"), label_style),
                        Span::raw("  "),
                        Span::styled(part, text_style),
                    ]
                } else {
                    vec![
                        Span::raw(" ".repeat(prefix_w)),
                        Span::styled(part, text_style),
                    ]
                }));
            }
        } else {
            entry_lines.push(Line::from(vec![
                Span::styled(marker, marker_style),
                Span::raw(" "),
                Span::styled(label, label_style),
            ]));
            for part in parts {
                entry_lines.push(Line::from(vec![
                    Span::raw(" ".repeat(prefix_w)),
                    Span::styled(part, text_style),
                ]));
            }
        }
        for line in entry_lines {
            lines.push(line);
            if entry.undone {
                undone_lines += 1;
            } else {
                last_done_end = lines.len();
            }
        }
    }

    if protocol.is_empty() {
        let hint = Style::default().fg(pal.dim).add_modifier(Modifier::ITALIC);
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("  Noch keine Schritte.", hint)));
        lines.push(Line::from(Span::styled("  Probiere z. B.  a -2 1 2", hint)));
    }

    // Keep the most recent executed step in view, with a peek at undone steps.
    let height = usize::from(area.height);
    let total = lines.len();
    let max_offset = total.saturating_sub(height);
    let offset = (last_done_end + undone_lines.min(height / 3))
        .saturating_sub(height)
        .min(max_offset);

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .scroll((u16::try_from(offset).unwrap_or(0), 0)),
        area,
    );
    if total > height {
        let mut state = ScrollbarState::new(max_offset).position(offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_symbol("┃")
                .thumb_style(Style::default().fg(pal.accent))
                .track_symbol(Some("│"))
                .track_style(Style::default().fg(pal.border)),
            area,
            &mut state,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_word_boundaries() {
        assert_eq!(
            wrap_words("Zeile 1 wird mit Zeile 2 vertauscht.", 14),
            vec!["Zeile 1 wird", "mit Zeile 2", "vertauscht."]
        );
        assert_eq!(wrap_words("kurz", 40), vec!["kurz"]);
        assert!(wrap_words("", 10).is_empty());
        assert!(wrap_words("   ", 10).is_empty());
    }

    #[test]
    fn splits_overlong_words() {
        assert_eq!(wrap_words("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap_words("ab abcdefgh", 4), vec!["ab", "abcd", "efgh"]);
    }

    #[test]
    fn never_exceeds_the_width() {
        let text = "Das (-123456789/7)-fache von Zeile 3 wird zu Zeile 1 addiert.";
        for width in 1..30 {
            for line in wrap_words(text, width) {
                assert!(line.chars().count() <= width, "{line:?} at {width}");
            }
        }
    }

    #[test]
    fn counts_umlauts_as_one_column() {
        assert_eq!(wrap_words("Übung äöü ßß", 5), vec!["Übung", "äöü", "ßß"]);
    }
}
