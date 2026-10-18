//! Modal overlays: help, solution confirmation, reference solution and check result.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph};

use crate::app::{App, Overlay};
use crate::check::{CheckReport, Diff};
use crate::echelon::Solution;
use crate::help::SECTIONS;
use crate::notation::matrix_label;
use crate::rational::to_text;

use super::matrix_view::{Brackets, Decor, EntryStyle, Geometry, Spacing, draw};
use super::palette::Palette;
use super::protocol::wrap_words;

/// Largest number of differing entries listed per comparison.
const MAX_DIFFS: usize = 8;
/// Width of the key column of the help screen.
const KEY_WIDTH: usize = 27;
/// Horizontal space taken by the popup border and padding.
const CHROME_W: u16 = 6;

/// Draws the open overlay, if any. `scroll` is clamped to the content.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    pal: &Palette,
    brackets: Brackets,
    scroll: &mut u16,
) {
    match app.overlay() {
        Overlay::None => {}
        Overlay::Help => scrollable(frame, area, " Hilfe ", 110, scroll, pal, |w| {
            help_lines(pal, w)
        }),
        Overlay::ConfirmSolution => confirm_solution(frame, area, pal),
        Overlay::Solution {
            from_step,
            solution,
        } => {
            let title = format!(
                " Musterlösung · ausgehend von {} ",
                matrix_label(*from_step)
            );
            scrollable(frame, area, &title, 110, scroll, pal, |w| {
                solution_lines(solution, *from_step, pal, brackets, w)
            });
        }
        Overlay::Check { at_step, report } => {
            let title = format!(" Kontrolle · Vergleich mit {} ", matrix_label(*at_step));
            scrollable(frame, area, &title, 110, scroll, pal, |w| {
                check_lines(report, *at_step, pal, w)
            });
        }
    }
}

fn popup_block<'a>(title: &'a str, footer: &'a str, pal: &Palette) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(pal.accent))
        .title(Span::styled(
            title,
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(
            Line::from(Span::styled(footer, Style::default().fg(pal.dim)))
                .alignment(Alignment::Right),
        )
        .padding(Padding::new(2, 2, 1, 0))
        .style(Style::default().bg(pal.surface).fg(pal.fg))
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// A popup whose content is built for the available inner width and may scroll.
fn scrollable(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    max_width: u16,
    scroll: &mut u16,
    pal: &Palette,
    build: impl FnOnce(usize) -> Vec<Line<'static>>,
) {
    let width = max_width.min(area.width.saturating_sub(4));
    let lines = build(usize::from(width.saturating_sub(CHROME_W)));
    let wanted = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_add(3);
    let rect = centered(area, width, wanted.min(area.height.saturating_sub(2)));
    let visible = usize::from(rect.height.saturating_sub(3));
    let overflow = lines.len() > visible;
    let footer = if overflow {
        " ↑↓ blättern · Esc schließt "
    } else {
        " Esc schließt "
    };

    frame.render_widget(Clear, rect);
    let block = popup_block(title, footer, pal);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let max_scroll =
        u16::try_from(lines.len().saturating_sub(usize::from(inner.height))).unwrap_or(u16::MAX);
    *scroll = (*scroll).min(max_scroll);
    frame.render_widget(Paragraph::new(lines).scroll((*scroll, 0)), inner);
}

/// `lead` followed by `text` wrapped to `width`; continuation lines are indented by `lead_width`.
fn hanging(
    lead: Vec<Span<'static>>,
    lead_width: usize,
    text: &str,
    style: Style,
    width: usize,
) -> Vec<Line<'static>> {
    let parts = wrap_words(text, width.saturating_sub(lead_width).max(10));
    let mut lead = Some(lead);
    parts
        .into_iter()
        .map(|part| {
            let mut spans = lead
                .take()
                .unwrap_or_else(|| vec![Span::raw(" ".repeat(lead_width))]);
            spans.push(Span::styled(part, style));
            Line::from(spans)
        })
        .collect()
}

fn plain_wrapped(text: &str, style: Style, width: usize) -> Vec<Line<'static>> {
    hanging(Vec::new(), 0, text, style, width)
}

fn help_lines(pal: &Palette, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (i, section) in SECTIONS.iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(Span::styled(
            section.title,
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        )));
        for (keys, text) in section.rows {
            let lead = vec![Span::styled(
                format!("  {keys:<KEY_WIDTH$}"),
                Style::default().fg(pal.accent2),
            )];
            lines.extend(hanging(
                lead,
                2 + KEY_WIDTH,
                text,
                Style::default().fg(pal.fg),
                width,
            ));
        }
    }
    lines
}

fn confirm_solution(frame: &mut Frame<'_>, area: Rect, pal: &Palette) {
    let rect = centered(area, 62.min(area.width.saturating_sub(4)), 9);
    frame.render_widget(Clear, rect);
    let block = popup_block(" Musterlösung anzeigen? ", "", pal);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let key = |k: &'static str| {
        Span::styled(
            k,
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        )
    };
    let mut lines = plain_wrapped(
        "Die Musterlösung verrät den gesamten Rechenweg. Mit einem Tipp (F2) übst du mehr – \
         möchtest du sie trotzdem sehen?",
        Style::default().fg(pal.fg),
        usize::from(inner.width),
    );
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        key("[J]"),
        Span::raw(" ja, anzeigen      "),
        key("[N]"),
        Span::raw(" nein, weiterüben"),
    ]));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn solution_lines(
    solution: &Solution,
    from_step: usize,
    pal: &Palette,
    brackets: Brackets,
    width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let from = matrix_label(from_step);
    if solution.steps.is_empty() {
        lines.extend(plain_wrapped(
            &format!("{from} steht bereits in Treppennormalform – es ist nichts zu tun."),
            Style::default().fg(pal.green),
            width,
        ));
    } else {
        lines.extend(plain_wrapped(
            &format!(
                "Gauß-Jordan von {from} aus in {} Schritten: Pivots von links nach rechts, \
                 bevorzugt eine 1 als Pivot, danach Nullen über und unter dem Pivot.",
                solution.steps.len()
            ),
            Style::default().fg(pal.muted),
            width,
        ));
        lines.push(Line::raw(""));
        let cmd_w = solution
            .steps
            .iter()
            .map(|s| s.op.to_command().chars().count())
            .max()
            .unwrap_or(0);
        for (i, step) in solution.steps.iter().enumerate() {
            let lead = vec![
                Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(pal.dim)),
                Span::styled(
                    format!("{:<cmd_w$}", step.op.to_command()),
                    Style::default()
                        .fg(pal.accent2)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
            ];
            lines.extend(hanging(
                lead,
                4 + cmd_w + 3,
                &step.op.describe(),
                Style::default().fg(pal.fg),
                width,
            ));
        }
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "Ergebnis (Treppennormalform):",
        Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
    )));
    let stacked = Geometry::new(
        &solution.result,
        EntryStyle::Stacked,
        Spacing::new(0),
        false,
        false,
    );
    let geometry = if stacked.width + 2 <= width && stacked.height <= 10 {
        stacked
    } else {
        Geometry::new(
            &solution.result,
            EntryStyle::Inline,
            Spacing::new(0),
            false,
            false,
        )
    };
    lines.extend(draw(
        &solution.result,
        &geometry,
        &Decor::default(),
        pal,
        brackets,
        2,
    ));
    lines
}

/// "1 Eintrag" / "n Einträge" (nominative).
fn entries(n: usize) -> String {
    if n == 1 {
        "1 Eintrag".to_owned()
    } else {
        format!("{n} Einträge")
    }
}

/// "1 Eintrag" / "n Einträgen" (dative).
fn entries_dat(n: usize) -> String {
    if n == 1 {
        "1 Eintrag".to_owned()
    } else {
        format!("{n} Einträgen")
    }
}

fn diff_lines(diffs: &[Diff], reveal_expected: bool, pal: &Palette) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = diffs
        .iter()
        .take(MAX_DIFFS)
        .map(|d| {
            let mut spans = vec![
                Span::styled(
                    format!("   Zeile {}, Spalte {}: ", d.row + 1, d.col + 1),
                    Style::default().fg(pal.muted),
                ),
                Span::styled(
                    format!("du hast {}", to_text(&d.entered)),
                    Style::default().fg(pal.red),
                ),
            ];
            if reveal_expected {
                spans.push(Span::styled(
                    format!(", richtig wäre {}", to_text(&d.expected)),
                    Style::default().fg(pal.green),
                ));
            }
            Line::from(spans)
        })
        .collect();
    if diffs.len() > MAX_DIFFS {
        lines.push(Line::from(Span::styled(
            format!("   … und {} weitere", diffs.len() - MAX_DIFFS),
            Style::default().fg(pal.dim),
        )));
    }
    lines
}

fn check_lines(
    report: &CheckReport,
    at_step: usize,
    pal: &Palette,
    width: usize,
) -> Vec<Line<'static>> {
    let label = matrix_label(at_step);
    let verdict = |ok: bool, text: &str| {
        let color = if ok { pal.green } else { pal.red };
        let lead = vec![Span::styled(
            if ok { "✓ " } else { "✗ " },
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )];
        hanging(lead, 2, text, Style::default().fg(color), width)
    };
    let note = |text: &str| plain_wrapped(text, Style::default().fg(pal.muted), width);

    let mut lines = Vec::new();
    if !report.sizes_match() {
        let (er, ec) = report.entered_size;
        let (xr, xc) = report.expected_size;
        lines.extend(verdict(
            false,
            &format!("Falsche Größe: Du hast {er}×{ec} eingegeben, {label} ist {xr}×{xc}."),
        ));
        return lines;
    }
    match (report.matches_current, report.matches_rref) {
        (true, true) => {
            lines.extend(verdict(
                true,
                &format!("Stimmt mit {label} überein – und das ist bereits die Treppennormalform."),
            ));
        }
        (true, false) => {
            lines.extend(verdict(true, &format!("Stimmt mit {label} überein.")));
            lines.extend(note(
                "Das ist noch nicht die Treppennormalform – rechne weiter.",
            ));
        }
        (false, true) => {
            lines.extend(verdict(
                true,
                "Das ist die Treppennormalform dieser Aufgabe.",
            ));
            lines.extend(note(&format!(
                "Sie unterscheidet sich von {label}: Du bist also schon weiter als der Trainer."
            )));
        }
        (false, false) => {
            lines.extend(verdict(
                false,
                &format!("Stimmt weder mit {label} noch mit der Treppennormalform überein."),
            ));
        }
    }
    if !report.matches_current && !report.matches_rref {
        // Detail the reference that is closer to what was entered; the other is summarised.
        let rref_closer = report.diff_rref.len() < report.diff_current.len();
        lines.push(Line::raw(""));
        if rref_closer {
            lines.extend(note(&format!(
                "Am nächsten liegt die Treppennormalform. Abweichungen ({}; die richtigen Werte bleiben verborgen):",
                entries(report.diff_rref.len())
            )));
            lines.extend(diff_lines(&report.diff_rref, false, pal));
            lines.push(Line::raw(""));
            lines.extend(note(&format!(
                "Von {label} weicht deine Matrix in {} ab.",
                entries_dat(report.diff_current.len())
            )));
        } else {
            lines.extend(note(&format!(
                "Am nächsten liegt {label}. Abweichungen ({}):",
                entries(report.diff_current.len())
            )));
            lines.extend(diff_lines(&report.diff_current, true, pal));
            lines.push(Line::raw(""));
            lines.extend(note(&format!(
                "Von der Treppennormalform weicht deine Matrix in {} ab.",
                entries_dat(report.diff_rref.len())
            )));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Theme;
    use crate::check::check;
    use crate::matrix::Matrix;
    use crate::parse::parse_matrix;
    use crate::ui::palette::ColorDepth;

    fn text(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn pal() -> Palette {
        Palette::new(Theme::Dark, ColorDepth::TrueColor)
    }

    #[test]
    fn check_overlay_hides_the_expected_normal_form_entries() {
        let a = Matrix::course_example();
        let entered = parse_matrix("1 0 0 9; 0 1 0 -4/7; 0 0 1 -10/7").unwrap();
        let out = text(&check_lines(&check(&entered, &a), 0, &pal(), 80));
        assert!(
            out.contains("weder mit A₀ noch mit der Treppennormalform"),
            "{out}"
        );
        assert!(out.contains("Zeile 1, Spalte 4: du hast 9"));
        assert!(
            out.contains("Am nächsten liegt die Treppennormalform"),
            "{out}"
        );
        assert!(out.contains("Abweichungen (1 Eintrag;"), "{out}");
        assert!(
            out.contains("Von A₀ weicht deine Matrix in 11 Einträgen ab."),
            "{out}"
        );
        assert!(!out.contains("15/7"), "the answer must stay hidden:\n{out}");
        assert!(!out.contains("richtig wäre"), "{out}");
    }

    #[test]
    fn check_overlay_reports_success_and_size_errors() {
        let a = Matrix::course_example();
        let r = parse_matrix("1 0 0 15/7; 0 1 0 -4/7; 0 0 1 -10/7").unwrap();
        let out = text(&check_lines(&check(&r, &a), 0, &pal(), 80));
        assert!(out.contains("Treppennormalform dieser Aufgabe"), "{out}");
        let out = text(&check_lines(&check(&a, &a), 0, &pal(), 80));
        assert!(out.contains("Stimmt mit A₀ überein"), "{out}");
        let small = parse_matrix("1 2").unwrap();
        let out = text(&check_lines(&check(&small, &a), 2, &pal(), 80));
        assert!(
            out.contains("Falsche Größe") && out.contains("1×2") && out.contains("3×4"),
            "{out}"
        );
    }

    #[test]
    fn a_matrix_close_to_the_current_one_is_compared_with_it() {
        let a = Matrix::course_example();
        let entered = parse_matrix("1 -2 3 -1; 2 -1 2 5; 3 1 2 3").unwrap();
        let out = text(&check_lines(&check(&entered, &a), 3, &pal(), 80));
        assert!(
            out.contains("Am nächsten liegt A₃. Abweichungen (1 Eintrag)"),
            "{out}"
        );
        assert!(
            out.contains("Zeile 2, Spalte 4: du hast 5, richtig wäre 2"),
            "{out}"
        );
        assert!(
            out.contains("Von der Treppennormalform weicht deine Matrix in"),
            "{out}"
        );
    }

    #[test]
    fn long_diff_lists_are_truncated() {
        let a = parse_matrix("1 2 3; 4 5 6; 7 8 10").unwrap();
        let entered = parse_matrix("9 9 9; 9 9 9; 9 9 9").unwrap();
        let out = text(&check_lines(&check(&entered, &a), 0, &pal(), 80));
        assert!(out.contains("… und 1 weitere"), "{out}");
    }

    #[test]
    fn overlay_text_wraps_to_the_width() {
        let a = Matrix::course_example();
        let entered = parse_matrix("1 0 0 9; 0 1 0 -4/7; 0 0 1 -10/7").unwrap();
        for width in [60, 70, 80] {
            for line in check_lines(&check(&entered, &a), 0, &pal(), width) {
                assert!(
                    line.width() <= width,
                    "{width}: {}",
                    text(std::slice::from_ref(&line))
                );
            }
            for line in help_lines(&pal(), width) {
                assert!(
                    line.width() <= width,
                    "{width}: {}",
                    text(std::slice::from_ref(&line))
                );
            }
        }
    }

    #[test]
    fn solution_overlay_lists_commands_and_the_result() {
        let a = Matrix::course_example();
        let sol = crate::echelon::solve(&a);
        let out = text(&solution_lines(&sol, 0, &pal(), Brackets::Tall, 80));
        assert!(out.contains("a -2 1 2"), "{out}");
        assert!(out.contains("Das (-2)-fache von Zeile 1 wird zu Zeile 2 addiert."));
        assert!(out.contains("Ergebnis (Treppennormalform):"));
        assert!(out.contains("15"));
    }

    #[test]
    fn solved_matrices_get_a_short_solution() {
        let done = parse_matrix("1 0; 0 1").unwrap();
        let sol = crate::echelon::solve(&done);
        let out = text(&solution_lines(&sol, 4, &pal(), Brackets::Tall, 80));
        assert!(
            out.contains("A₄ steht bereits in Treppennormalform"),
            "{out}"
        );
    }

    #[test]
    fn help_mentions_every_section() {
        let out = text(&help_lines(&pal(), 100));
        for section in SECTIONS {
            assert!(out.contains(section.title));
        }
    }
}
