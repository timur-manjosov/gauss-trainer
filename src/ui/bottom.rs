//! The bottom area: status line, feedback, command bar and key hints.

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, Prompt, Tone, format_clock};

use super::palette::Palette;
use super::protocol::wrap_words;

/// The analysis part of the status line; `compact` shortens the wording.
fn analysis_spans(app: &App, pal: &Palette, compact: bool) -> Vec<Span<'static>> {
    let analysis = app.analysis();
    let mut spans = vec![
        Span::styled(
            format!("Rang r = {}", analysis.rank()),
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(pal.dim)),
    ];
    if analysis.is_rref() {
        spans.push(Span::styled(
            "Treppennormalform erreicht ✓",
            Style::default().fg(pal.green).add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::styled("noch offen: ", Style::default().fg(pal.muted)));
        spans.push(Span::styled(
            analysis.missing(compact).join(" · "),
            Style::default().fg(pal.amber),
        ));
    }
    spans
}

fn width_of(spans: &[Span<'_>]) -> usize {
    spans.iter().map(Span::width).sum()
}

/// Draws the one-line status bar.
pub fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App, pal: &Palette) {
    let bar = Style::default().bg(pal.bar).fg(pal.fg);
    let total = usize::from(area.width);
    let mut right: Vec<Span<'static>> = Vec::new();

    if !app.is_exam() {
        let m = app.matrix();
        let origin = app
            .seed()
            .map_or_else(|| "eigene Matrix".to_owned(), |s| format!("Seed {s}"));
        let preset = if app.seed().is_some() {
            format!(" · {}", app.preset().label())
        } else {
            String::new()
        };
        right.push(Span::styled(
            format!("{}×{}{preset} · {origin} ", m.rows(), m.cols()),
            Style::default().fg(pal.dim),
        ));
    }

    let mut left: Vec<Span<'static>> = vec![Span::raw(" ")];
    if let Some(elapsed) = app.exam_elapsed() {
        left.push(Span::styled(
            "Prüfungsmodus",
            Style::default().fg(pal.amber).add_modifier(Modifier::BOLD),
        ));
        left.push(Span::styled(
            format!("  ·  Schritt {}", app.step()),
            Style::default().fg(pal.muted),
        ));
        right.push(Span::styled(
            format!("⏱ {} ", format_clock(elapsed)),
            Style::default().fg(pal.amber).add_modifier(Modifier::BOLD),
        ));
    } else if app.show_analysis() {
        // Prefer the full wording with the matrix info, then shorter variants.
        let full = analysis_spans(app, pal, false);
        let compact = analysis_spans(app, pal, true);
        let right_w = width_of(&right);
        let fits = |spans: &[Span<'_>], with_right: bool| {
            1 + width_of(spans) + if with_right { right_w + 2 } else { 0 } <= total
        };
        if fits(&full, true) {
            left.extend(full);
        } else if fits(&compact, true) {
            left.extend(compact);
        } else {
            right.clear();
            left.extend(if fits(&full, false) { full } else { compact });
        }
    } else {
        left.push(Span::styled(
            "Live-Analyse aus (Ctrl+L)",
            Style::default().fg(pal.dim),
        ));
    }

    let left_w = width_of(&left);
    let right_w = width_of(&right);
    let mut spans = left;
    if left_w + right_w < total {
        spans.push(Span::raw(" ".repeat(total - left_w - right_w)));
        spans.extend(right);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(bar), area);
}

/// Draws the (up to two lines of) feedback message.
pub fn render_feedback(frame: &mut Frame<'_>, area: Rect, app: &App, pal: &Palette) {
    let Some(feedback) = app.feedback() else {
        return;
    };
    let (icon, color) = match feedback.tone {
        Tone::Info => ("›", pal.muted),
        Tone::Success => ("✓", pal.green),
        Tone::Hint => ("◆", pal.accent2),
        Tone::Error => ("✗", pal.red),
    };
    let width = usize::from(area.width).saturating_sub(3);
    let mut wrapped = wrap_words(&feedback.text, width);
    let max_lines = usize::from(area.height).max(1);
    if wrapped.len() > max_lines {
        wrapped.truncate(max_lines);
        if let Some(last) = wrapped.last_mut() {
            let keep = width.saturating_sub(1);
            *last = last.chars().take(keep).collect::<String>() + "…";
        }
    }
    let lines: Vec<Line<'static>> = wrapped
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let lead = if i == 0 {
                Span::styled(
                    format!(" {icon} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("   ")
            };
            let text_color = if feedback.tone == Tone::Info {
                pal.muted
            } else {
                color
            };
            Line::from(vec![
                lead,
                Span::styled(text, Style::default().fg(text_color)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

/// Draws the bordered command bar and places the cursor.
pub fn render_command(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    pal: &Palette,
    show_cursor: bool,
) {
    let (title, placeholder) = match app.prompt() {
        Prompt::Command => (
            " Befehl ",
            "m 1 -1/2  ·  a 8 1 2  ·  s 1 2  ·  ? für die Hilfe",
        ),
        Prompt::NewMatrix => (
            " Eigene Matrix · Zeilen mit ; trennen · Esc bricht ab ",
            "z. B.  1 -2 3; 2 -1 2",
        ),
        Prompt::CheckMatrix => (
            " Deine Matrix prüfen · Zeilen mit ; trennen · Esc bricht ab ",
            "z. B.  1 0 15/7; 0 1 -4/7",
        ),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(pal.accent))
        .title(Span::styled(
            title,
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(pal.surface));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt = Span::styled(
        "❯ ",
        Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
    );
    let editor = app.editor();
    let text_w = usize::from(inner.width).saturating_sub(3).max(1);
    let line = if editor.is_empty() {
        Line::from(vec![
            prompt,
            Span::styled(placeholder, Style::default().fg(pal.dim)),
        ])
    } else {
        let skip = (editor.cursor() + 1).saturating_sub(text_w);
        let visible: String = editor.text().chars().skip(skip).take(text_w).collect();
        Line::from(vec![
            prompt,
            Span::styled(visible, Style::default().fg(pal.fg)),
        ])
    };
    frame.render_widget(
        Paragraph::new(line),
        Rect {
            x: inner.x + 1,
            width: inner.width.saturating_sub(1),
            ..inner
        },
    );

    if show_cursor {
        let skip = (editor.cursor() + 1).saturating_sub(text_w);
        let col = u16::try_from(editor.cursor() - skip).unwrap_or(0);
        frame.set_cursor_position(Position::new(inner.x + 3 + col, inner.y));
    }
}

/// Shortcuts on the left of the bottom line, most important first.
const HINTS: &[(&str, &str)] = &[
    ("Enter", "ausführen"),
    ("↑↓", "Verlauf"),
    ("Ctrl+Z", "Zurück"),
    ("Ctrl+Y", "Vor"),
    ("Ctrl+R", "Reset"),
    ("F2", "Tipp"),
    ("F3", "Prüfen"),
    ("Ctrl+N", "Neu"),
    ("Ctrl+T", "Thema"),
];

/// Shortcuts that always stay visible at the right edge.
const FIXED_HINTS: &[(&str, &str)] = &[("?", "Hilfe"), ("Ctrl+Q", "Ende")];

fn hint_spans(
    items: &[(&str, &str)],
    width: usize,
    pal: &Palette,
    skip_assistance: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut used = 0;
    for (key, label) in items {
        if skip_assistance && matches!(*key, "F2" | "F3") {
            continue;
        }
        let need = key.chars().count() + 1 + label.chars().count() + 2;
        if used + need > width {
            break;
        }
        spans.push(Span::styled(
            (*key).to_owned(),
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}  "),
            Style::default().fg(pal.dim),
        ));
        used += need;
    }
    spans
}

/// Draws the key hints; `?` and `Ctrl+Q` stay put, the rest drops from the end.
pub fn render_hints(frame: &mut Frame<'_>, area: Rect, app: &App, pal: &Palette) {
    let width = usize::from(area.width);
    let fixed = hint_spans(FIXED_HINTS, width, pal, false);
    let fixed_w = width_of(&fixed);
    let mut spans = vec![Span::raw(" ")];
    spans.extend(hint_spans(
        HINTS,
        width.saturating_sub(fixed_w + 2),
        pal,
        app.is_exam(),
    ));
    let used = width_of(&spans);
    if used + fixed_w <= width {
        spans.push(Span::raw(" ".repeat(width - used - fixed_w)));
        spans.extend(fixed);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
