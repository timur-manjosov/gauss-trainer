//! Rendering. This module only draws the state of an [`App`]; it never
//! changes it. Presentation-only state (scroll positions, colour depth,
//! bracket style) lives in [`View`].

mod bottom;
pub mod matrix_view;
mod overlay;
pub mod palette;
mod protocol;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::app::{App, MatrixLayout, Overlay};
use crate::notation::matrix_label;

use matrix_view::{Brackets, Decor, EntryStyle, Geometry, Spacing, draw};
use palette::{ColorDepth, Palette};

/// Smallest supported terminal width.
pub const MIN_WIDTH: u16 = 80;
/// Smallest supported terminal height.
pub const MIN_HEIGHT: u16 = 24;
/// The Rechenprotokoll never gets less than this many columns.
const MIN_PROTOCOL_WIDTH: u16 = 30;

/// Presentation state that is not part of the application logic.
#[derive(Debug, Clone)]
pub struct View {
    /// Horizontal scroll offset of the matrix panel.
    pub scroll_x: u16,
    /// Vertical scroll offset of the matrix panel.
    pub scroll_y: u16,
    /// Scroll offset of the open overlay.
    pub overlay_scroll: u16,
    /// Colour depth of the terminal.
    pub depth: ColorDepth,
    /// Bracket glyphs.
    pub brackets: Brackets,
}

impl View {
    /// Creates a view with no scrolling.
    #[must_use]
    pub fn new(depth: ColorDepth, brackets: Brackets) -> Self {
        Self {
            scroll_x: 0,
            scroll_y: 0,
            overlay_scroll: 0,
            depth,
            brackets,
        }
    }

    /// Scrolls the matrix panel; offsets are clamped when drawing.
    pub fn scroll(&mut self, dx: i32, dy: i32) {
        self.scroll_x = self
            .scroll_x
            .saturating_add_signed(i16::try_from(dx).unwrap_or(0));
        self.scroll_y = self
            .scroll_y
            .saturating_add_signed(i16::try_from(dy).unwrap_or(0));
    }

    /// Scrolls the open overlay; the offset is clamped when drawing.
    pub fn scroll_overlay(&mut self, dy: i32) {
        self.overlay_scroll = self
            .overlay_scroll
            .saturating_add_signed(i16::try_from(dy).unwrap_or(0));
    }
}

/// Draws the whole interface.
pub fn render(frame: &mut Frame<'_>, app: &App, view: &mut View) {
    let area = frame.area();
    let pal = Palette::new(app.theme(), view.depth);
    frame.render_widget(
        Block::default().style(Style::default().bg(pal.bg).fg(pal.fg)),
        area,
    );

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(frame, area, &pal);
        return;
    }

    let root = area.inner(Margin::new(1, 0));
    let [main, status, feedback, command, hints] = Layout::vertical([
        Constraint::Min(8),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(root);

    let left_w = matrix_panel_width(app, main.width);
    let [left, right] =
        Layout::horizontal([Constraint::Length(left_w), Constraint::Fill(1)]).areas(main);

    render_matrix_panel(frame, left, app, view, &pal);

    let steps = app.step();
    let protocol_block = panel(
        Line::from(Span::styled(" Rechenprotokoll ", title_style(&pal))),
        &pal,
    )
    .title(
        Line::from(Span::styled(
            format!(
                " {steps} {} ",
                if steps == 1 { "Schritt" } else { "Schritte" }
            ),
            Style::default().fg(pal.dim),
        ))
        .alignment(Alignment::Right),
    );
    let inner = protocol_block.inner(right);
    frame.render_widget(protocol_block, right);
    protocol::render(frame, inner, app, &pal);

    bottom::render_status(frame, status, app, &pal);
    bottom::render_feedback(frame, feedback, app, &pal);
    let overlay_open = *app.overlay() != Overlay::None;
    bottom::render_command(frame, command, app, &pal, !overlay_open);
    bottom::render_hints(frame, hints, app, &pal);

    if overlay_open {
        // Push the main view into the background before the modal panel appears.
        let dim = Style::default().add_modifier(Modifier::DIM);
        for cell in &mut frame.buffer_mut().content {
            cell.set_style(dim);
        }
    }
    overlay::render(
        frame,
        area,
        app,
        &pal,
        view.brackets,
        &mut view.overlay_scroll,
    );
}

fn title_style(pal: &Palette) -> Style {
    Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)
}

fn panel<'a>(title: Line<'a>, pal: &Palette) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(pal.border))
        .style(Style::default().bg(pal.surface).fg(pal.fg))
        .title(title)
        .padding(Padding::horizontal(1))
}

fn render_too_small(frame: &mut Frame<'_>, area: Rect, pal: &Palette) {
    let lines = vec![
        Line::from(Span::styled(
            "Das Terminal ist zu klein",
            Style::default().fg(pal.amber).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            format!(
                "Aktuell {}×{} – benötigt werden mindestens {MIN_WIDTH}×{MIN_HEIGHT} Zeichen.",
                area.width, area.height
            ),
            Style::default().fg(pal.fg),
        )),
        Line::from(Span::styled(
            "Bitte vergrößere das Fenster. Ctrl+Q beendet das Programm.",
            Style::default().fg(pal.dim),
        )),
    ];
    let height = u16::try_from(lines.len()).unwrap_or(4).min(area.height);
    let rect = Rect {
        y: area.y + (area.height - height) / 2,
        height,
        ..area
    };
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), rect);
}

/// Whether the zero-row annotation has to be reserved in the matrix panel.
fn needs_annotation(app: &App) -> bool {
    app.show_analysis() && !app.analysis().zero_rows().is_empty()
}

/// Width of the matrix panel: 58 % of the main area, more if the matrix needs it.
fn matrix_panel_width(app: &App, main_width: u16) -> u16 {
    let wanted = Geometry::new(
        app.matrix(),
        EntryStyle::Stacked,
        Spacing::new(0),
        true,
        needs_annotation(app),
    )
    .width;
    // borders and padding take four columns
    let wanted = u16::try_from(wanted + 4).unwrap_or(u16::MAX);
    let base = u16::try_from(u32::from(main_width) * 58 / 100).unwrap_or(main_width);
    base.max(wanted.min(main_width.saturating_sub(MIN_PROTOCOL_WIDTH)))
}

/// Picks the entry style and spacing for the available space.
fn choose_geometry(app: &App, avail_w: usize, avail_h: usize) -> Geometry {
    let m = app.matrix();
    let annotate = needs_annotation(app);
    let fits = |g: &Geometry| g.width <= avail_w && g.height <= avail_h;
    let build = |style, rows, cols| Geometry::new(m, style, Spacing { rows, cols }, true, annotate);
    // Blank lines between the rows if there is room for them.
    let airy = |style, cols| {
        let spaced = build(style, 1, cols);
        if fits(&spaced) {
            spaced
        } else {
            build(style, 0, cols)
        }
    };
    match app.layout() {
        MatrixLayout::Stacked => airy(EntryStyle::Stacked, 3),
        MatrixLayout::Inline => airy(EntryStyle::Inline, 3),
        MatrixLayout::Auto => [(EntryStyle::Stacked, 3), (EntryStyle::Inline, 3)]
            .into_iter()
            .map(|(style, cols)| airy(style, cols))
            .find(|g| fits(g))
            .unwrap_or_else(|| airy(EntryStyle::Inline, 2)),
    }
}

fn render_matrix_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    view: &mut View,
    pal: &Palette,
) {
    let title = Line::from(vec![
        Span::styled(" Matrix ", title_style(pal)),
        Span::styled("· ", Style::default().fg(pal.dim)),
        Span::styled(
            format!("{} ", matrix_label(app.step())),
            Style::default()
                .fg(pal.accent2)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let block = panel(title, pal).title(
        Line::from(Span::styled(
            format!(" {}×{} ", app.matrix().rows(), app.matrix().cols()),
            Style::default().fg(pal.dim),
        ))
        .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let analysis = app.show_analysis().then(|| app.analysis());
    let legend_h = u16::from(analysis.is_some());
    let view_rect = Rect {
        height: inner.height.saturating_sub(legend_h),
        ..inner
    };

    let geometry = choose_geometry(
        app,
        usize::from(view_rect.width),
        usize::from(view_rect.height),
    );
    let indent = usize::from(view_rect.width).saturating_sub(geometry.width) / 2;
    let free_h = usize::from(view_rect.height).saturating_sub(geometry.height);
    let top_pad = free_h / 3;

    let changed: &[usize] = if app.show_changes() {
        app.changed_rows()
    } else {
        &[]
    };
    let decor = Decor {
        analysis,
        changed,
        labels: true,
    };
    let mut lines: Vec<Line<'static>> = vec![Line::raw(""); top_pad];
    lines.extend(draw(
        app.matrix(),
        &geometry,
        &decor,
        pal,
        view.brackets,
        indent,
    ));

    let content_w = indent + geometry.width;
    let max_y = lines.len().saturating_sub(usize::from(view_rect.height));
    let max_x = content_w.saturating_sub(usize::from(view_rect.width));
    view.scroll_y = view.scroll_y.min(u16::try_from(max_y).unwrap_or(u16::MAX));
    view.scroll_x = view.scroll_x.min(u16::try_from(max_x).unwrap_or(u16::MAX));

    frame.render_widget(
        Paragraph::new(lines).scroll((view.scroll_y, view.scroll_x)),
        view_rect,
    );

    let track = Style::default().fg(pal.border);
    let thumb = Style::default().fg(pal.accent);
    if max_y > 0 {
        let mut state = ScrollbarState::new(max_y).position(usize::from(view.scroll_y));
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_symbol("┃")
                .track_symbol(Some("│"))
                .thumb_style(thumb)
                .track_style(track),
            view_rect,
            &mut state,
        );
    }
    if max_x > 0 {
        let mut state = ScrollbarState::new(max_x).position(usize::from(view.scroll_x));
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_symbol("━")
                .track_symbol(Some("─"))
                .thumb_style(thumb)
                .track_style(track),
            view_rect,
            &mut state,
        );
    }

    if legend_h == 1 {
        let legend_rect = Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        };
        frame.render_widget(
            Paragraph::new(legend(pal, usize::from(inner.width))),
            legend_rect,
        );
    }
}

/// The colour legend; entries that do not fit the width are dropped.
fn legend(pal: &Palette, width: usize) -> Line<'static> {
    let dim = Style::default().fg(pal.dim);
    let items = [
        ("■", pal.green, "Pivot = 1"),
        ("■", pal.amber, "Pivot ≠ 1"),
        ("■", pal.red, "muss 0 werden"),
        ("▌", pal.accent, "Pivotspalte"),
    ];
    let mut spans = Vec::new();
    let mut used = 0;
    for (symbol, color, label) in items {
        let need = 2 + label.chars().count() + 2;
        if used + need > width {
            break;
        }
        spans.push(Span::styled(
            format!("{symbol} "),
            Style::default().fg(color),
        ));
        spans.push(Span::styled(format!("{label}  "), dim));
        used += need;
    }
    Line::from(spans)
}
