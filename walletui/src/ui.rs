use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, Field, Mode};

const BG: Color = Color::Rgb(12, 13, 15);
const PANEL: Color = Color::Rgb(22, 24, 28);
const PANEL_HI: Color = Color::Rgb(36, 39, 45);
const CYAN: Color = Color::Rgb(118, 185, 176);
const MAGENTA: Color = Color::Rgb(160, 139, 181);
const YELLOW: Color = Color::Rgb(198, 169, 105);
const GREEN: Color = Color::Rgb(128, 176, 133);
const BLUE: Color = Color::Rgb(128, 157, 194);
const RED: Color = Color::Rgb(194, 116, 116);
const TEXT: Color = Color::Rgb(211, 215, 222);
const MUTED: Color = Color::Rgb(128, 134, 145);
const SHIMMER_SOFT: Color = Color::Rgb(166, 183, 194);
const SHIMMER_MID: Color = Color::Rgb(205, 214, 218);
const SHIMMER_HOT: Color = Color::Rgb(238, 241, 235);

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BG).fg(TEXT)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(5),
        ])
        .split(area);

    render_header(app, frame, chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(chunks[1]);
    render_list(app, frame, body[0]);
    render_detail(app, frame, body[1]);

    render_footer(app, frame, chunks[2]);
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let mode = mode_label(&app.mode);
    let category = if app.category_filter.is_empty() {
        "ALL".to_string()
    } else {
        app.category_filter.clone()
    };
    let query = if app.query.is_empty() {
        "-".to_string()
    } else {
        app.query.clone()
    };

    let lines = vec![
        title_line(app.animation_tick),
        Line::from(vec![
            pill("MODE", mode, BLUE),
            Span::raw("  "),
            pill("CAT", &category, CYAN),
            Span::raw("  "),
            pill("FIND", &query, GREEN),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(TEXT).bg(PANEL))
            .block(panel_block("COMMAND BRIDGE", CYAN)),
        area,
    );
}

fn render_list(app: &App, frame: &mut Frame, area: Rect) {
    let filtered = app.filtered_indices();
    let visible_rows = area.height.saturating_sub(2).max(1) as usize;
    let selected = app.selected.min(filtered.len().saturating_sub(1));
    let start = if selected >= visible_rows {
        selected + 1 - visible_rows
    } else {
        0
    };
    let end = (start + visible_rows).min(filtered.len());
    let items = filtered
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
        .map(|(row, index)| {
            let entry = &app.entries[*index];
            let is_selected = row == selected;
            let marker = if is_selected { ">>" } else { "  " };
            let base = if is_selected {
                Style::default()
                    .fg(TEXT)
                    .bg(PANEL_HI)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT).bg(PANEL)
            };
            let category_style = if is_selected {
                Style::default()
                    .fg(CYAN)
                    .bg(PANEL_HI)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(CYAN).bg(PANEL)
            };
            let user_style = if is_selected {
                Style::default().fg(MUTED).bg(PANEL_HI)
            } else {
                Style::default().fg(MUTED).bg(PANEL)
            };

            let line = if is_selected {
                let text = format!(
                    "{marker} [{}] {}  @{}",
                    entry.category, entry.title, entry.username
                );
                highlight_text(&text, app.animation_tick / 2, TEXT, CYAN, PANEL_HI, true)
            } else {
                Line::from(vec![
                    Span::styled(format!("{marker} "), base),
                    Span::styled(format!("[{}] ", entry.category), category_style),
                    Span::styled(entry.title.clone(), base),
                    Span::styled(format!("  @{}", entry.username), user_style),
                ])
            };

            ListItem::new(line).style(base)
        })
        .collect::<Vec<_>>();

    let position = if filtered.is_empty() { 0 } else { selected + 1 };
    let title = format!(
        "PASSWORD DECK {position}/{}  VIEW {}-{}  TOTAL {}",
        filtered.len(),
        if filtered.is_empty() { 0 } else { start + 1 },
        end,
        app.entries.len()
    );
    frame.render_widget(
        List::new(items)
            .style(Style::default().fg(TEXT).bg(PANEL))
            .block(panel_block(&title, MAGENTA)),
        area,
    );
}

fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
    match app.mode {
        Mode::Editing { field, .. } => render_form(app, frame, area, field),
        _ => render_selected(app, frame, area),
    }
}

fn render_selected(app: &App, frame: &mut Frame, area: Rect) {
    let lines = if let Some(entry) = app.selected_entry() {
        let password = if app.show_password {
            entry.password.clone()
        } else if entry.password.is_empty() {
            String::new()
        } else {
            "*".repeat(entry.password.chars().count().max(8))
        };
        vec![
            field_line("分类", &entry.category, CYAN, false),
            field_line("名称", &entry.title, BLUE, false),
            field_line("账号", &entry.username, GREEN, false),
            field_line("密码", &password, RED, false),
            field_line("备注", &entry.notes, BLUE, false),
            Line::raw(""),
            Line::from(vec![
                Span::styled("SECURITY ", Style::default().fg(YELLOW).bg(PANEL)),
                Span::styled(
                    if app.show_password {
                        "PASSWORD VISIBLE"
                    } else {
                        "PASSWORD MASKED"
                    },
                    Style::default()
                        .fg(if app.show_password { RED } else { CYAN })
                        .bg(PANEL)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![Span::styled(
                "NO RECORDS LOADED",
                Style::default()
                    .fg(YELLOW)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "Press a to create your first encrypted password.",
                Style::default().fg(MUTED).bg(PANEL),
            )]),
        ]
    };

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(TEXT).bg(PANEL))
            .block(panel_block("ACCESS PANEL", CYAN))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_form(app: &App, frame: &mut Frame, area: Rect, active_field: usize) {
    let values = [
        (Field::Category, &app.form.category, CYAN),
        (Field::Title, &app.form.title, BLUE),
        (Field::Username, &app.form.username, GREEN),
        (Field::Password, &app.form.password, RED),
        (Field::Notes, &app.form.notes, BLUE),
    ];
    let lines = values
        .iter()
        .enumerate()
        .map(|(index, (field, value, color))| {
            field_line(field.label(), value, *color, index == active_field)
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(TEXT).bg(PANEL))
            .block(panel_block("EDIT TERMINAL", YELLOW))
            .wrap(Wrap { trim: false }),
        area,
    );
    if let Some((_, value, _)) = values.get(active_field) {
        set_form_cursor(frame, area, active_field, value, app.form_cursor);
    }
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let menu = match app.mode {
        Mode::Normal => vec![
            ("A", "新增", GREEN),
            ("E", "编辑", CYAN),
            ("D", "删除", RED),
            ("/", "搜索", BLUE),
            ("C", "分类", YELLOW),
            ("V", "显隐", BLUE),
            ("MOUSE", "复制", GREEN),
            ("Q", "退出", MUTED),
        ],
        Mode::Search => vec![
            ("TYPE", "搜索", BLUE),
            ("ENTER", "完成", GREEN),
            ("ESC", "返回", YELLOW),
            ("BACK", "删除", RED),
        ],
        Mode::Category => vec![
            ("TYPE", "分类", CYAN),
            ("ENTER", "完成", GREEN),
            ("ESC", "返回", YELLOW),
            ("BACK", "删除", RED),
        ],
        Mode::Editing { .. } => vec![
            ("TAB", "字段", CYAN),
            ("ENTER", "保存", GREEN),
            ("ESC", "取消", YELLOW),
            ("BACK", "删除", RED),
        ],
        Mode::ConfirmDelete => vec![("Y", "确认删除", RED), ("N/ESC", "取消", GREEN)],
    };

    let mut menu_line = Vec::new();
    for (key, label, color) in menu {
        if !menu_line.is_empty() {
            menu_line.push(Span::raw("  "));
        }
        menu_line.push(keycap(key, color));
        menu_line.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(TEXT).bg(PANEL),
        ));
    }

    let status = if app.status.is_empty() {
        "SYSTEM READY".to_string()
    } else {
        app.status.clone()
    };
    let mut status_spans = vec![
        Span::styled(
            "STATUS ",
            Style::default()
                .fg(YELLOW)
                .bg(PANEL_HI)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];
    status_spans
        .extend(highlight_text(&status, app.animation_tick / 3, TEXT, CYAN, PANEL, false).spans);
    let lines = vec![Line::from(menu_line), Line::from(status_spans)];

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(TEXT).bg(PANEL))
            .block(panel_block("HOTKEY LOADOUT", BLUE)),
        area,
    );
}

fn field_line(label: &str, value: &str, accent: Color, active: bool) -> Line<'static> {
    let label_style = if active {
        Style::default()
            .fg(TEXT)
            .bg(PANEL_HI)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(accent).bg(PANEL)
    };
    let value_style = Style::default().fg(TEXT).bg(PANEL);
    Line::from(vec![
        Span::styled(format!(" {label:<6} "), label_style),
        Span::styled(" ", Style::default().bg(PANEL)),
        Span::styled(value.to_string(), value_style),
    ])
}

fn set_form_cursor(frame: &mut Frame, area: Rect, active_field: usize, value: &str, cursor: usize) {
    if area.width <= 2 || area.height <= 2 {
        return;
    }

    let input_start = area.x.saturating_add(12);
    let max_x = area.x.saturating_add(area.width.saturating_sub(2));
    let cursor_x = input_start
        .saturating_add(cursor.min(value.chars().count()) as u16)
        .min(max_x);
    let cursor_y = area.y.saturating_add(1 + active_field as u16);
    if cursor_y < area.y.saturating_add(area.height.saturating_sub(1)) {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn mode_label(mode: &Mode) -> &'static str {
    match mode {
        Mode::Normal => "BROWSE",
        Mode::Search => "SEARCH",
        Mode::Category => "CATEGORY",
        Mode::Editing { .. } => "EDIT",
        Mode::ConfirmDelete => "DELETE",
    }
}

fn title_line(tick: u64) -> Line<'static> {
    let mut spans = highlight_text(
        " WALLETUI / LOCAL VAULT ",
        tick / 2,
        TEXT,
        CYAN,
        PANEL_HI,
        true,
    )
    .spans;
    spans.push(Span::styled(
        " encrypted local password deck",
        Style::default().fg(MUTED).bg(PANEL),
    ));
    Line::from(spans)
}

fn highlight_text(
    text: &str,
    tick: u64,
    normal_fg: Color,
    highlight_fg: Color,
    bg: Color,
    bold: bool,
) -> Line<'static> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Line::default();
    }

    let band_width = 7usize;
    let head = (tick as usize) % (chars.len() + band_width);
    let spans = chars
        .into_iter()
        .enumerate()
        .map(|(index, ch)| {
            let distance = index.abs_diff(head);
            let fg = shimmer_color(distance, normal_fg, highlight_fg);
            let mut style = Style::default().fg(fg).bg(bg);
            if bold || distance <= 3 {
                style = style.add_modifier(Modifier::BOLD);
            }
            Span::styled(ch.to_string(), style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

fn shimmer_color(distance: usize, normal: Color, accent: Color) -> Color {
    match distance {
        0 => SHIMMER_HOT,
        1 => SHIMMER_MID,
        2 => accent,
        3 => SHIMMER_SOFT,
        _ => normal,
    }
}

fn pill(label: &'static str, value: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {label}:{value} "),
        Style::default()
            .fg(color)
            .bg(PANEL_HI)
            .add_modifier(Modifier::BOLD),
    )
}

fn keycap(key: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {key} "),
        Style::default()
            .fg(color)
            .bg(PANEL_HI)
            .add_modifier(Modifier::BOLD),
    )
}

fn panel_block(title: &str, border: Color) -> Block<'_> {
    Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(border)
                .bg(PANEL)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border).bg(PANEL))
        .style(Style::default().fg(TEXT).bg(PANEL))
}
