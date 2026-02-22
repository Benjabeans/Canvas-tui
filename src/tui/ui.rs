use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use super::{App, AssignmentSort, CalendarItem, SubmissionState, Tab, UnifiedViewMode};
use crate::models::Assignment;
use chrono::{Datelike, Local, NaiveDate, Utc};
use std::collections::BTreeMap;

// ─── Palette ─────────────────────────────────────────────────────────────────

/// Primary amber accent — titles, selected markers, focal items.
const AMBER: Color = Color::Rgb(255, 185, 50);
/// Softer amber for secondary labels (tab numbers, column headers).
const AMBER_SOFT: Color = Color::Rgb(170, 120, 35);
/// Primary content text — warm off-white.
const TEXT: Color = Color::Rgb(232, 222, 205);
/// Secondary text — due dates, sub-labels.
const TEXT_DIM: Color = Color::Rgb(108, 98, 82);
/// Very muted — borders, separators, empty markers.
const TEXT_MUTED: Color = Color::Rgb(58, 52, 42);
/// Selected row background (warm dark gold).
const SEL_BG: Color = Color::Rgb(52, 42, 18);
/// Focal (next actionable) item background.
const FOCAL_BG: Color = Color::Rgb(62, 42, 0);
/// Focal item foreground — same amber as the primary accent.
const FOCAL: Color = Color::Rgb(255, 185, 50);
/// Status bar / header background.
const HDR_BG: Color = Color::Rgb(16, 14, 11);
/// Good scores, submitted state.
const SUCCESS: Color = Color::Rgb(125, 195, 95);
/// Warnings, imminent deadlines.
const CAUTION: Color = Color::Rgb(255, 162, 38);
/// Errors, missing assignments.
const DANGER: Color = Color::Rgb(210, 68, 58);
/// Informational — submitted-but-not-graded state.
const INFO: Color = Color::Rgb(98, 172, 238);

// ─── Spinner ─────────────────────────────────────────────────────────────────

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner_char(frame: u64) -> &'static str {
    SPINNER[frame as usize % SPINNER.len()]
}

// ─── Countdown Timer ─────────────────────────────────────────────────────────

/// Returns a human-readable countdown string and a color that progresses from
/// green (≥7 days) → yellow (1–7 days) → orange (<24h) → red (<6h) → bold red (<1h).
fn countdown_timer(due: chrono::DateTime<Utc>) -> (String, Color) {
    let now = Utc::now();
    let remaining = due.signed_duration_since(now);

    if remaining.num_seconds() <= 0 {
        return ("Past due".into(), DANGER);
    }

    let total_mins = remaining.num_minutes();
    let days = remaining.num_days();
    let hours = (total_mins / 60) % 24;
    let mins = total_mins % 60;

    let text = if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    };

    let color = if days >= 7 {
        SUCCESS                           // ≥ 1 week — green
    } else if days >= 3 {
        Color::Rgb(200, 210, 80)          // 3–7 days — yellow-green
    } else if days >= 1 {
        CAUTION                           // 1–3 days — orange/yellow
    } else if hours >= 6 {
        Color::Rgb(240, 120, 40)          // 6–24h — deep orange
    } else {
        DANGER                            // < 6h — red
    };

    (text, color)
}

// ─── Main render ─────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_tabs(f, app, chunks[0]);
    render_clock(f, chunks[0]);

    match app.active_tab {
        Tab::Dashboard => render_dashboard(f, app, chunks[1]),
        Tab::Courses => render_courses(f, app, chunks[1]),
        Tab::Assignments => render_schedule(f, app, chunks[1]),
        Tab::Announcements => render_announcements(f, app, chunks[1]),
    }

    render_status_bar(f, app, chunks[2]);
}

// ─── Tab Bar ─────────────────────────────────────────────────────────────────

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let active = *tab == app.active_tab;
            Line::from(vec![
                Span::styled(
                    format!(" {} ", i + 1),
                    Style::default().fg(if active { AMBER_SOFT } else { TEXT_MUTED }),
                ),
                Span::styled(
                    format!("{} ", tab.title()),
                    Style::default()
                        .fg(if active { TEXT } else { TEXT_DIM })
                        .add_modifier(if active { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ])
        })
        .collect();

    let selected = Tab::ALL.iter().position(|t| *t == app.active_tab).unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(TEXT_MUTED))
                .title(" ◈ Canvas TUI ")
                .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        )
        .select(selected)
        .divider(Span::styled(" │ ", Style::default().fg(TEXT_MUTED)))
        .highlight_style(
            Style::default()
                .fg(AMBER)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );

    f.render_widget(tabs, area);
}

// ─── Clock ───────────────────────────────────────────────────────────────────

fn render_clock(f: &mut Frame, tab_area: Rect) {
    let time_str = format!(" {} ", Local::now().format("%a %b %d  %H:%M:%S"));
    let w = time_str.len() as u16;
    let clock_area = Rect {
        x: tab_area.right().saturating_sub(w),
        y: tab_area.y,
        width: w.min(tab_area.width),
        height: 1,
    };
    f.render_widget(
        Paragraph::new(time_str).style(Style::default().fg(TEXT_DIM)),
        clock_area,
    );
}

// ─── Status Bar ──────────────────────────────────────────────────────────────

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let sync_hint = app
        .cached_at
        .map(|t| format!("   synced {}", t.with_timezone(&Local).format("%b %d %H:%M")))
        .unwrap_or_default();

    let (indicator, ind_color) = if app.loading {
        (spinner_char(app.frame_count), CAUTION)
    } else {
        ("●", SUCCESS)
    };

    let hints = if app.submission_state.is_hidden() {
        let nav = match (app.active_tab, app.unified_view_mode) {
            (Tab::Assignments, UnifiedViewMode::CalendarView) =>
                "   │   q quit   Tab switch   j/k nav   v list-view   Enter submit   t today   r refresh",
            (Tab::Assignments, UnifiedViewMode::ListView) =>
                "   │   q quit   Tab switch   j/k nav   v cal-view   s sort   f filter   Enter submit   r refresh",
            _ =>
                "   │   q quit   Tab switch   j/k nav   r refresh",
        };
        format!("{}{}  ", nav, sync_hint)
    } else {
        "   │   j/k navigate   Space/Enter select   Esc back   y confirm   n cancel  "
            .to_string()
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", indicator),
            Style::default().fg(ind_color).bg(HDR_BG),
        ),
        Span::styled(
            app.status_message.as_str(),
            Style::default().fg(TEXT).bg(HDR_BG),
        ),
        Span::styled(hints, Style::default().fg(TEXT_MUTED).bg(HDR_BG)),
    ]))
    .style(Style::default().bg(HDR_BG));

    f.render_widget(bar, area);
}

// ─── Dashboard ───────────────────────────────────────────────────────────────

fn render_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    // ── Overview panel ────────────────────────────────────────────────────
    let user_name = app
        .user
        .as_ref()
        .and_then(|u| u.name.clone())
        .unwrap_or_else(|| "Student".into());
    let unread_count = app
        .announcements
        .iter()
        .filter(|a| a.read_state.as_deref() == Some("unread"))
        .count();
    let upcoming_count = app.calendar_events.len();

    let overview = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(AMBER)),
            Span::styled(
                format!("Welcome back, {}.", user_name),
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("     ●  ", Style::default().fg(AMBER_SOFT)),
            Span::styled(
                format!("{} courses enrolled", app.courses.len()),
                Style::default().fg(TEXT),
            ),
            Span::styled("     ○  ", Style::default().fg(TEXT_DIM)),
            Span::styled(
                format!("{} upcoming events", upcoming_count),
                Style::default().fg(TEXT_DIM),
            ),
            Span::styled("     ", Style::default()),
            Span::styled(
                if unread_count > 0 { "●  " } else { "○  " },
                Style::default().fg(if unread_count > 0 { DANGER } else { TEXT_MUTED }),
            ),
            Span::styled(
                format!(
                    "{} unread announcement{}",
                    unread_count,
                    if unread_count == 1 { "" } else { "s" }
                ),
                Style::default().fg(if unread_count > 0 { DANGER } else { TEXT_DIM }),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(" Overview ")
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    f.render_widget(overview, chunks[0]);

    // ── Bottom split: Upcoming list (left) + Detail (right) ──────────────
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[1]);

    render_upcoming_assignments(f, app, bottom[0]);
    render_dashboard_detail(f, app, bottom[1]);
}

fn render_upcoming_assignments(f: &mut Frame, app: &mut App, area: Rect) {
    let now = Utc::now();
    let today = now.date_naive();
    let one_month = now + chrono::Duration::days(30);
    let focal_id = app.focal_assignment_id;

    let mut upcoming: Vec<(&str, &Assignment)> = app
        .assignments
        .iter()
        .flat_map(|(course, assignments)| assignments.iter().map(move |a| (course.as_str(), a)))
        .filter(|(_, a)| {
            a.due_at
                .map(|d| d.date_naive() >= today && d <= one_month)
                .unwrap_or(false)
        })
        .collect();

    upcoming.sort_by(|a, b| match (a.1.due_at, b.1.due_at) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, _) => std::cmp::Ordering::Greater,
        (_, None) => std::cmp::Ordering::Less,
        (Some(x), Some(y)) => x.cmp(&y),
    });

    app.dashboard_list_state.set_len(upcoming.len());

    let items: Vec<ListItem> = if upcoming.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  ○  Nothing due in the next 30 days",
            Style::default().fg(TEXT_DIM),
        )))]
    } else {
        upcoming
            .iter()
            .enumerate()
            .map(|(idx, (course_name, a))| {
                let is_selected = idx == app.dashboard_list_state.selected;
                let is_focal = Some(a.id) == focal_id;

                let bg = if is_selected {
                    SEL_BG
                } else if is_focal {
                    FOCAL_BG
                } else {
                    Color::Reset
                };

                let name = a.name.as_deref().unwrap_or("Unnamed");
                let is_today = a.due_at.map(|d| d.date_naive() == today).unwrap_or(false);
                let due = a
                    .due_at
                    .map(|d| {
                        if is_today {
                            format!("Today {}", d.format("%H:%M"))
                        } else {
                            d.format("%b %d").to_string()
                        }
                    })
                    .unwrap_or_default();
                let (status, status_color) = assignment_status(a);

                let (timer_text, timer_color) = a
                    .due_at
                    .map(|d| countdown_timer(d))
                    .unwrap_or_default();

                // " ▶ " = 3 chars, timer + trailing space
                let prefix_len = 3;
                let timer_display = format!(" {} ", timer_text);
                let timer_len = timer_display.len();
                let avail = (area.width as usize).saturating_sub(prefix_len + timer_len + 2);
                let name_trunc = if name.len() > avail {
                    format!("{}…", &name[..avail.saturating_sub(1)])
                } else {
                    name.to_string()
                };
                let pad = avail.saturating_sub(name_trunc.len());

                let (marker, marker_fg) = if is_selected {
                    ("▶", AMBER)
                } else if is_focal {
                    ("»", FOCAL)
                } else {
                    (" ", TEXT_MUTED)
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" {} ", marker),
                            Style::default().fg(marker_fg).bg(bg),
                        ),
                        Span::styled(
                            name_trunc,
                            Style::default()
                                .fg(TEXT)
                                .bg(bg)
                                .add_modifier(
                                    if is_selected || is_focal {
                                        Modifier::BOLD
                                    } else {
                                        Modifier::empty()
                                    },
                                ),
                        ),
                        Span::styled(
                            " ".repeat(pad),
                            Style::default().bg(bg),
                        ),
                        Span::styled(
                            timer_display,
                            Style::default().fg(timer_color).bg(bg),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("    ", Style::default().bg(bg)),
                        Span::styled(
                            format!("{:<14}", due),
                            Style::default()
                                .fg(if is_today { CAUTION } else { TEXT_DIM })
                                .bg(bg)
                                .add_modifier(
                                    if is_today {
                                        Modifier::BOLD
                                    } else {
                                        Modifier::empty()
                                    },
                                ),
                        ),
                        Span::styled(
                            format!(" {}", status),
                            Style::default().fg(status_color).bg(bg),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("    ", Style::default().bg(bg)),
                        Span::styled(
                            course_name.to_string(),
                            Style::default().fg(TEXT_MUTED).bg(bg),
                        ),
                    ]),
                ])
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(format!(" Upcoming ({}) ", upcoming.len()))
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    app.dashboard_list_state
        .inner
        .select(Some(app.dashboard_list_state.selected));
    f.render_stateful_widget(list, area, &mut app.dashboard_list_state.inner);
}

fn render_dashboard_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_MUTED))
        .title(" Assignment Detail ")
        .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD));

    let Some((course_name, assignment)) = app.get_selected_dashboard_assignment() else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Select an assignment to view details.",
                Style::default().fg(TEXT_DIM),
            )))
            .block(detail_block),
            area,
        );
        return;
    };

    let name = assignment.name.as_deref().unwrap_or("Unnamed");
    let now = Utc::now();
    let today = now.date_naive();

    let due_str = assignment
        .due_at
        .map(|d| {
            let formatted = d.format("%B %d, %Y at %H:%M").to_string();
            if d.date_naive() == today {
                format!("{formatted}  (Today)")
            } else if d < now {
                format!("{formatted}  (Past due)")
            } else {
                formatted
            }
        })
        .unwrap_or_else(|| "No due date".into());

    let points_str = assignment
        .points_possible
        .map(|p| format!("{p} pts"))
        .unwrap_or_else(|| "─".into());

    let types_str = assignment
        .submission_types
        .as_ref()
        .map(|t| t.join(", "))
        .unwrap_or_else(|| "─".into());

    let (status, status_color) = assignment_status(assignment);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {name}"),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let label_style = Style::default().fg(AMBER_SOFT);
    let value_style = Style::default().fg(TEXT);

    let fields: Vec<(&str, String, Style)> = {
        let mut f = vec![
            ("Course", course_name.to_string(), value_style),
            ("Due", due_str, value_style),
            ("Points", points_str, value_style),
            ("Types", types_str, value_style),
            ("Status", status.clone(), Style::default().fg(status_color)),
        ];

        if let Some(ref sub) = assignment.submission {
            if let Some(score) = sub.score {
                let pts = assignment.points_possible.unwrap_or(0.0);
                f.push((
                    "Score",
                    format!("{score:.1} / {pts}"),
                    Style::default().fg(SUCCESS),
                ));
            }
            if let Some(grade) = sub.grade.as_deref() {
                if sub.score.is_none() {
                    f.push(("Grade", grade.to_string(), Style::default().fg(SUCCESS)));
                }
            }
            if let Some(submitted) = sub.submitted_at {
                f.push((
                    "Submitted",
                    submitted.format("%B %d, %Y at %H:%M").to_string(),
                    value_style,
                ));
            }
            if let Some(graded) = sub.graded_at {
                f.push((
                    "Graded",
                    graded.format("%B %d, %Y").to_string(),
                    value_style,
                ));
            }
            if let Some(attempt) = sub.attempt {
                f.push(("Attempt", attempt.to_string(), value_style));
            }
            if let Some(late) = sub.late {
                let (text, color) = if late {
                    ("Yes", DANGER)
                } else {
                    ("No", value_style.fg.unwrap_or(TEXT))
                };
                f.push(("Late", text.to_string(), Style::default().fg(color)));
            }
            if let Some(missing) = sub.missing {
                if missing {
                    f.push(("Missing", "Yes".to_string(), Style::default().fg(DANGER)));
                }
            }
        }

        f
    };

    for (label, value, style) in &fields {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<10}", label), label_style),
            Span::styled(value.as_str(), *style),
        ]));
    }

    if let Some(ref desc) = assignment.description {
        let stripped = strip_html(desc);
        if !stripped.trim().is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ── Description ──────────────────────────────",
                Style::default().fg(TEXT_MUTED),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", stripped.trim()),
                Style::default().fg(TEXT_DIM),
            )));
        }
    }

    if let Some(ref url) = assignment.html_url {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ── Link ─────────────────────────────────────",
            Style::default().fg(TEXT_MUTED),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {url}"),
            Style::default().fg(INFO),
        )));
    }

    let detail = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(detail_block);

    f.render_widget(detail, area);
}

// ─── Submission Modal ─────────────────────────────────────────────────────────

fn render_submission_modal(f: &mut Frame, app: &mut App, area: Rect) {
    match &app.submission_state {
        SubmissionState::TypePicker => render_type_picker(f, app, area),
        SubmissionState::UrlInput => render_text_input_modal(
            f,
            area,
            " Submit URL ",
            "Enter the URL to submit:",
            &app.submission_input,
            "Enter to confirm  ·  Esc to go back",
        ),
        SubmissionState::FileInput => render_text_input_modal(
            f,
            area,
            " Submit File ",
            "Enter the full file path:",
            &app.submission_input,
            "Enter to confirm  ·  Esc to go back",
        ),
        SubmissionState::TextPreview => render_text_preview(f, app, area),
        SubmissionState::Confirming => render_confirm_modal(f, app, area),
        SubmissionState::Submitting => render_submitting_modal(f, app, area),
        SubmissionState::Done { success, message } => {
            render_done_modal(f, area, *success, message.clone())
        }
        SubmissionState::Hidden => {}
    }
}

fn popup_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width.saturating_sub(w)) / 2,
        area.y + (area.height.saturating_sub(h)) / 2,
        w,
        h,
    )
}

fn render_type_picker(f: &mut Frame, app: &mut App, area: Rect) {
    let kinds = &app.submission_supported_kinds;
    let h = (kinds.len() as u16 + 6).min(area.height.saturating_sub(2));
    let w = 54u16.min(area.width.saturating_sub(4));
    let popup = popup_rect(w, h, area);

    f.render_widget(Clear, popup);

    let items: Vec<ListItem> = kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let is_sel = i == app.submission_cursor;
            let bg = if is_sel { SEL_BG } else { Color::Reset };
            let (marker, mfg) = if is_sel { ("▶", AMBER) } else { (" ", TEXT_MUTED) };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {marker} "),
                    Style::default().fg(mfg).bg(bg),
                ),
                Span::styled(
                    kind.label(),
                    Style::default()
                        .fg(if is_sel { TEXT } else { TEXT_DIM })
                        .bg(bg)
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ]))
        })
        .collect();

    let mut state = app.filter_list_state.inner.clone();
    state.select(Some(app.submission_cursor));

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(AMBER_SOFT))
            .title(" Submit Assignment ")
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD))
            .title_bottom(Line::from(vec![
                Span::styled(" j/k ", Style::default().fg(AMBER_SOFT)),
                Span::styled("move  ", Style::default().fg(TEXT_DIM)),
                Span::styled("Enter ", Style::default().fg(AMBER_SOFT)),
                Span::styled("select  ", Style::default().fg(TEXT_DIM)),
                Span::styled("Esc ", Style::default().fg(AMBER_SOFT)),
                Span::styled("cancel ", Style::default().fg(TEXT_DIM)),
            ])),
    );

    f.render_stateful_widget(list, popup, &mut state);
}

fn render_text_input_modal(
    f: &mut Frame,
    area: Rect,
    title: &str,
    prompt: &str,
    input: &str,
    footer: &str,
) {
    let popup = popup_rect(70, 8, area);
    f.render_widget(Clear, popup);

    // Truncate from the left if input is too wide for the box
    let inner_w = popup.width.saturating_sub(4) as usize;
    let display = if input.len() > inner_w {
        format!("…{}", &input[input.len().saturating_sub(inner_w - 1)..])
    } else {
        input.to_string()
    };
    let cursor_line = format!("{display}_");

    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(prompt, Style::default().fg(TEXT_DIM))),
        Line::from(""),
        Line::from(Span::styled(cursor_line, Style::default().fg(TEXT).add_modifier(Modifier::BOLD))),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(AMBER_SOFT))
            .title(title)
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD))
            .title_bottom(Line::from(Span::styled(
                format!(" {footer} "),
                Style::default().fg(TEXT_DIM),
            ))),
    );

    f.render_widget(para, popup);
}

fn render_text_preview(f: &mut Frame, app: &App, area: Rect) {
    let popup = popup_rect(72, 22, area);
    f.render_widget(Clear, popup);

    let content = &app.submission_input;
    let inner_w = popup.width.saturating_sub(4) as usize;
    let max_lines = popup.height.saturating_sub(8) as usize;

    let preview_lines: Vec<Line> = content
        .lines()
        .take(max_lines)
        .map(|l| {
            let truncated = if l.len() > inner_w {
                format!("{}…", &l[..inner_w.saturating_sub(1)])
            } else {
                l.to_string()
            };
            Line::from(Span::styled(
                format!("  {truncated}"),
                Style::default().fg(TEXT_DIM),
            ))
        })
        .collect();

    let total_lines = content.lines().count();
    let truncation_note = if total_lines > max_lines {
        format!("  … ({} more lines not shown)", total_lines - max_lines)
    } else {
        String::new()
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Preview of text to submit:",
            Style::default().fg(AMBER_SOFT),
        )),
        Line::from(Span::styled(
            "  ──────────────────────────────────────────────────────────",
            Style::default().fg(TEXT_MUTED),
        )),
    ];
    lines.extend(preview_lines);
    if !truncation_note.is_empty() {
        lines.push(Line::from(Span::styled(
            truncation_note,
            Style::default().fg(TEXT_MUTED),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────────────────────────────────────────",
        Style::default().fg(TEXT_MUTED),
    )));
    lines.push(Line::from(vec![
        Span::styled("  Submit this text?  ", Style::default().fg(TEXT)),
        Span::styled("y ", Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled("yes  ", Style::default().fg(TEXT_DIM)),
        Span::styled("n ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)),
        Span::styled("no / re-edit", Style::default().fg(TEXT_DIM)),
    ]));

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(AMBER_SOFT))
                .title(" Text Entry — Confirm Submission ")
                .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        );

    f.render_widget(para, popup);
}

fn render_confirm_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup = popup_rect(66, 12, area);
    f.render_widget(Clear, popup);

    let kind_label = match &app.submission_kind {
        Some(k) => k.label(),
        None => "Unknown",
    };

    let inner_w = popup.width.saturating_sub(6) as usize;
    let display_input = if app.submission_input.len() > inner_w {
        format!("…{}", &app.submission_input[app.submission_input.len().saturating_sub(inner_w - 1)..])
    } else {
        app.submission_input.clone()
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Type    ", Style::default().fg(AMBER_SOFT)),
            Span::styled(kind_label, Style::default().fg(TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Content ", Style::default().fg(AMBER_SOFT)),
            Span::styled(display_input, Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ────────────────────────────────────────────────────────",
            Style::default().fg(TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Submit this?  ", Style::default().fg(TEXT)),
            Span::styled("y ", Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("yes  ", Style::default().fg(TEXT_DIM)),
            Span::styled("n ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)),
            Span::styled("no / go back", Style::default().fg(TEXT_DIM)),
        ]),
    ];

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(AMBER_SOFT))
                .title(" Confirm Submission ")
                .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        );

    f.render_widget(para, popup);
}

fn render_submitting_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup = popup_rect(40, 5, area);
    f.render_widget(Clear, popup);

    let spin = spinner_char(app.frame_count);
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {spin}  "), Style::default().fg(CAUTION)),
            Span::styled("Submitting…", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CAUTION))
            .title(" Submitting ")
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    f.render_widget(para, popup);
}

fn render_done_modal(f: &mut Frame, area: Rect, success: bool, message: String) {
    let popup = popup_rect(62, 7, area);
    f.render_widget(Clear, popup);

    let (icon, border_color, msg_color) = if success {
        ("✓", SUCCESS, SUCCESS)
    } else {
        ("✗", DANGER, DANGER)
    };

    let inner_w = popup.width.saturating_sub(6) as usize;
    let wrapped: Vec<Line> = message
        .chars()
        .collect::<String>()
        .as_str()
        .chars()
        .collect::<Vec<_>>()
        .chunks(inner_w)
        .map(|chunk| {
            Line::from(Span::styled(
                format!("  {}", chunk.iter().collect::<String>()),
                Style::default().fg(msg_color),
            ))
        })
        .collect();

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {icon}  "), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
            Span::styled(
                if success { "Done" } else { "Error" },
                Style::default().fg(border_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];
    lines.extend(wrapped);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press any key to close",
        Style::default().fg(TEXT_MUTED),
    )));

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(if success { " Submitted " } else { " Submission Failed " })
                .title_style(
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                ),
        );

    f.render_widget(para, popup);
}

// ─── Course Filter Popup ─────────────────────────────────────────────────────

fn render_course_filter_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let course_names = app.assignment_course_names();
    let count = course_names.len();
    if count == 0 {
        return;
    }

    // Size the popup: width based on longest name, height based on item count
    let max_name_len = course_names.iter().map(|n| n.len()).max().unwrap_or(10);
    let popup_w = (max_name_len as u16 + 12).min(area.width.saturating_sub(4)); // " [x]  name "
    let popup_h = ((count as u16) + 4).min(area.height.saturating_sub(2)); // items + border + header + footer
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = course_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == app.filter_list_state.selected;
            let enabled = app.course_filter.is_empty() || app.course_filter.contains(*name);
            let bg = if is_selected { SEL_BG } else { Color::Reset };

            let (marker, marker_fg) = if is_selected {
                ("▶", AMBER)
            } else {
                (" ", TEXT_MUTED)
            };

            let checkbox = if enabled { "[●]" } else { "[ ]" };
            let check_color = if enabled { SUCCESS } else { TEXT_MUTED };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", marker),
                    Style::default().fg(marker_fg).bg(bg),
                ),
                Span::styled(
                    format!("{} ", checkbox),
                    Style::default().fg(check_color).bg(bg),
                ),
                Span::styled(
                    name.to_string(),
                    Style::default()
                        .fg(if enabled { TEXT } else { TEXT_DIM })
                        .bg(bg)
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]))
        })
        .collect();

    let filter_label = if app.course_filter.is_empty() {
        "all".to_string()
    } else {
        format!("{}/{}", app.course_filter.len(), count)
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(AMBER_SOFT))
            .title(format!(" Filter Courses ({}) ", filter_label))
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD))
            .title_bottom(Line::from(vec![
                Span::styled(" space", Style::default().fg(AMBER_SOFT)),
                Span::styled(" toggle  ", Style::default().fg(TEXT_DIM)),
                Span::styled("enter/esc", Style::default().fg(AMBER_SOFT)),
                Span::styled(" close ", Style::default().fg(TEXT_DIM)),
            ])),
    );

    app.filter_list_state
        .inner
        .select(Some(app.filter_list_state.selected));
    f.render_stateful_widget(list, popup_area, &mut app.filter_list_state.inner);
}

// ─── Courses ─────────────────────────────────────────────────────────────────

fn render_courses(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .courses
        .iter()
        .enumerate()
        .map(|(i, course)| {
            let name = course.name.as_deref().unwrap_or("Unnamed Course");
            let code = course.course_code.as_deref().unwrap_or("");
            let students = course
                .total_students
                .map(|n| format!("{n} students"))
                .unwrap_or_default();

            let is_selected = i == app.course_list_state.selected;
            let (marker, marker_fg) = if is_selected { ("▶", AMBER) } else { ("○", TEXT_MUTED) };
            let bg = if is_selected { SEL_BG } else { Color::Reset };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", marker), Style::default().fg(marker_fg).bg(bg)),
                Span::styled(
                    name,
                    Style::default()
                        .fg(TEXT)
                        .bg(bg)
                        .add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::styled(
                    format!("  {}", code),
                    Style::default().fg(AMBER_SOFT).bg(bg),
                ),
                Span::styled(
                    format!("   {}", students),
                    Style::default().fg(TEXT_MUTED).bg(bg),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(format!(" Courses ({}) ", app.courses.len()))
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    app.course_list_state.inner.select(Some(app.course_list_state.selected));
    f.render_stateful_widget(list, area, &mut app.course_list_state.inner);
}

// ─── Schedule (unified Calendar + Assignments) ────────────────────────────────

fn render_schedule(f: &mut Frame, app: &mut App, area: Rect) {
    match app.unified_view_mode {
        UnifiedViewMode::CalendarView => render_schedule_calendar(f, app, area),
        UnifiedViewMode::ListView => render_schedule_list(f, app, area),
    }
}

fn render_schedule_calendar(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    render_calendar_list(f, app, chunks[0]);
    render_schedule_calendar_detail(f, app, chunks[1]);

    if app.show_course_filter {
        render_course_filter_popup(f, app, area);
    }
    if !app.submission_state.is_hidden() {
        render_submission_modal(f, app, area);
    }
}

fn render_schedule_calendar_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_MUTED))
        .title(" Detail ")
        .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD));

    let Some(item) = app.calendar_items.get(app.calendar_list_state.selected) else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Select an item to view details.",
                Style::default().fg(TEXT_DIM),
            )))
            .block(detail_block),
            area,
        );
        return;
    };

    // If this CalendarItem is linked to an assignment, show full assignment detail.
    if let Some(assignment_id) = item.assignment_id {
        if let Some((course_name, assignment)) = app.get_assignment_by_id(assignment_id) {
            let asgn_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(TEXT_MUTED))
                .title(" Assignment Detail ")
                .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD));
            render_assignment_detail_for(f, area, asgn_block, course_name, assignment);
            return;
        }
    }

    // Fallback: lightweight calendar event detail.
    render_calendar_event_detail(f, area, detail_block, item);
}

// ─── Assignments ─────────────────────────────────────────────────────────────

fn assignment_status(a: &Assignment) -> (String, Color) {
    let now = Utc::now();
    if let Some(ref sub) = a.submission {
        match sub.workflow_state.as_deref() {
            Some("graded") => {
                let grade = sub
                    .score
                    .map(|s| format!("{s:.1}/{}", a.points_possible.unwrap_or(0.0)))
                    .unwrap_or_else(|| "Graded".into());
                (grade, SUCCESS)
            }
            Some("submitted") => ("Submitted".into(), INFO),
            _ => {
                if a.due_at.map_or(false, |d| d < now) {
                    if sub.missing.unwrap_or(false) {
                        ("Missing!".into(), DANGER)
                    } else {
                        ("Past due".into(), CAUTION)
                    }
                } else {
                    ("Not submitted".into(), TEXT_DIM)
                }
            }
        }
    } else if a.due_at.map_or(false, |d| d < now) {
        ("Past due".into(), CAUTION)
    } else {
        ("─".into(), TEXT_MUTED)
    }
}

fn assignment_status_priority(a: &Assignment) -> u8 {
    let now = Utc::now();
    if let Some(ref sub) = a.submission {
        match sub.workflow_state.as_deref() {
            Some("graded") => 4,
            Some("submitted") => 3,
            _ => {
                if a.due_at.map_or(false, |d| d < now) {
                    if sub.missing.unwrap_or(false) { 0 } else { 1 }
                } else {
                    2
                }
            }
        }
    } else if a.due_at.map_or(false, |d| d < now) {
        1
    } else {
        2
    }
}

fn render_schedule_list(f: &mut Frame, app: &mut App, area: Rect) {
    let sort_label = app.assignment_sort.label();
    let filter_hint = if app.course_filter.is_empty() {
        String::new()
    } else {
        format!("  filter: {} course{}", app.course_filter.len(),
            if app.course_filter.len() == 1 { "" } else { "s" })
    };
    let block_title = format!(" Schedule [List]   s: {}   f: filter{}   v: calendar ", sort_label, filter_hint);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    if app.assignment_sort == AssignmentSort::Course {
        render_assignments_grouped(f, app, chunks[0], &block_title);
    } else {
        render_assignments_flat(f, app, chunks[0], &block_title);
    }

    render_assignment_detail(f, app, chunks[1]);

    if app.show_course_filter {
        render_course_filter_popup(f, app, area);
    }

    if !app.submission_state.is_hidden() {
        render_submission_modal(f, app, area);
    }
}

fn render_assignments_grouped(f: &mut Frame, app: &mut App, area: Rect, block_title: &str) {
    let focal_id = app.focal_assignment_id;
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_idx = 0usize;
    let mut selected_item_idx = 0usize;

    for (course_name, assignments) in app.assignments.iter().filter(|(name, _)| app.course_passes_filter(name)) {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(" ◈  ", Style::default().fg(AMBER_SOFT)),
            Span::styled(
                course_name.as_str(),
                Style::default()
                    .fg(AMBER_SOFT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({} assignments) ", assignments.len()),
                Style::default().fg(TEXT_MUTED),
            ),
        ])));

        for assignment in assignments {
            let is_selected = flat_idx == app.assignment_list_state.selected;
            let is_focal = Some(assignment.id) == focal_id;

            if is_selected {
                selected_item_idx = items.len();
            }

            let (marker, marker_fg) = if is_selected {
                ("▶", AMBER)
            } else if is_focal {
                ("»", FOCAL)
            } else {
                (" ", TEXT_MUTED)
            };

            let bg = if is_selected {
                SEL_BG
            } else if is_focal {
                FOCAL_BG
            } else {
                Color::Reset
            };

            let name = assignment.name.as_deref().unwrap_or("Unnamed");
            let due = assignment
                .due_at
                .map(|d| d.format("%b %d  %H:%M").to_string())
                .unwrap_or_else(|| "No due date".into());
            let points = assignment
                .points_possible
                .map(|p| format!("{p} pts"))
                .unwrap_or_else(|| "─".into());
            let (status, status_color) = assignment_status(assignment);

            let name_style = Style::default().fg(TEXT).bg(bg).add_modifier(
                if is_focal && !is_selected { Modifier::BOLD } else { Modifier::empty() },
            );

            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", marker), Style::default().fg(marker_fg).bg(bg)),
                Span::styled(format!("{name:<40}"), name_style),
                Span::styled(format!(" {due:<18}"), Style::default().fg(TEXT_DIM).bg(bg)),
                Span::styled(format!(" {points:<10}"), Style::default().fg(TEXT_MUTED).bg(bg)),
                Span::styled(format!(" {status}"), Style::default().fg(status_color).bg(bg)),
            ])));

            flat_idx += 1;
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  ○  No assignments found.",
            Style::default().fg(TEXT_DIM),
        ))));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(block_title.to_string())
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    app.assignment_list_state.inner.select(Some(selected_item_idx));
    f.render_stateful_widget(list, area, &mut app.assignment_list_state.inner);
}

fn render_assignments_flat(f: &mut Frame, app: &mut App, area: Rect, block_title: &str) {
    let focal_id = app.focal_assignment_id;

    let mut flat: Vec<(&str, &Assignment)> = app
        .assignments
        .iter()
        .filter(|(name, _)| app.course_passes_filter(name))
        .flat_map(|(course, assignments)| assignments.iter().map(move |a| (course.as_str(), a)))
        .collect();

    match app.assignment_sort {
        AssignmentSort::DueDateAsc => flat.sort_by(|a, b| match (a.1.due_at, b.1.due_at) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
            (Some(x), Some(y)) => x.cmp(&y),
        }),
        AssignmentSort::DueDateDesc => flat.sort_by(|a, b| match (a.1.due_at, b.1.due_at) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
            (Some(x), Some(y)) => y.cmp(&x),
        }),
        AssignmentSort::Status => flat.sort_by_key(|(_, a)| assignment_status_priority(a)),
        AssignmentSort::Course => unreachable!(),
    }

    let mut items: Vec<ListItem> = Vec::new();
    for (idx, (course_name, assignment)) in flat.iter().enumerate() {
        let is_selected = idx == app.assignment_list_state.selected;
        let is_focal = Some(assignment.id) == focal_id;

        let (marker, marker_fg) = if is_selected {
            ("▶", AMBER)
        } else if is_focal {
            ("»", FOCAL)
        } else {
            (" ", TEXT_MUTED)
        };

        let bg = if is_selected {
            SEL_BG
        } else if is_focal {
            FOCAL_BG
        } else {
            Color::Reset
        };

        let name = assignment.name.as_deref().unwrap_or("Unnamed");
        let due = assignment
            .due_at
            .map(|d| d.format("%b %d  %H:%M").to_string())
            .unwrap_or_else(|| "No due date".into());
        let points = assignment
            .points_possible
            .map(|p| format!("{p} pts"))
            .unwrap_or_else(|| "─".into());
        let (status, status_color) = assignment_status(assignment);

        let name_style = Style::default().fg(TEXT).bg(bg).add_modifier(
            if is_focal && !is_selected { Modifier::BOLD } else { Modifier::empty() },
        );

        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!(" {} ", marker), Style::default().fg(marker_fg).bg(bg)),
            Span::styled(format!("{name:<36}"), name_style),
            Span::styled(format!(" {:<22}", course_name), Style::default().fg(TEXT_MUTED).bg(bg)),
            Span::styled(format!(" {due:<18}"), Style::default().fg(TEXT_DIM).bg(bg)),
            Span::styled(format!(" {points:<10}"), Style::default().fg(TEXT_MUTED).bg(bg)),
            Span::styled(format!(" {status}"), Style::default().fg(status_color).bg(bg)),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  ○  No assignments found.",
            Style::default().fg(TEXT_DIM),
        ))));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(block_title.to_string())
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    app.assignment_list_state
        .inner
        .select(Some(app.assignment_list_state.selected));
    f.render_stateful_widget(list, area, &mut app.assignment_list_state.inner);
}

fn render_assignment_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(TEXT_MUTED))
        .title(" Assignment Detail ")
        .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD));

    let Some((course_name, assignment)) = app.get_selected_assignment() else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Select an assignment to view details.",
                Style::default().fg(TEXT_DIM),
            )))
            .block(detail_block),
            area,
        );
        return;
    };

    render_assignment_detail_for(f, area, detail_block, course_name, assignment);
}

/// Shared assignment detail renderer. Accepts pre-fetched data so it can be
/// called from both the list view and the calendar view (where the selected
/// item is a CalendarItem backed by an assignment).
fn render_assignment_detail_for<'a>(
    f: &mut Frame,
    area: Rect,
    detail_block: Block<'a>,
    course_name: &str,
    assignment: &crate::models::Assignment,
) {
    let name = assignment.name.as_deref().unwrap_or("Unnamed");
    let now = Utc::now();
    let today = now.date_naive();

    let due_str = assignment
        .due_at
        .map(|d| {
            let formatted = d.format("%B %d, %Y at %H:%M").to_string();
            if d.date_naive() == today {
                format!("{formatted}  (Today)")
            } else if d < now {
                format!("{formatted}  (Past due)")
            } else {
                formatted
            }
        })
        .unwrap_or_else(|| "No due date".into());

    let points_str = assignment
        .points_possible
        .map(|p| format!("{p} pts"))
        .unwrap_or_else(|| "─".into());

    let types_str = assignment
        .submission_types
        .as_ref()
        .map(|t| t.join(", "))
        .unwrap_or_else(|| "─".into());

    let (status, status_color) = assignment_status(assignment);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {name}"),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let label_style = Style::default().fg(AMBER_SOFT);
    let value_style = Style::default().fg(TEXT);

    let fields: Vec<(&str, String, Style)> = {
        let mut flds = vec![
            ("Course", course_name.to_string(), value_style),
            ("Due", due_str, value_style),
            ("Points", points_str, value_style),
            ("Types", types_str, value_style),
            ("Status", status.clone(), Style::default().fg(status_color)),
        ];

        if let Some(ref sub) = assignment.submission {
            if let Some(score) = sub.score {
                let pts = assignment.points_possible.unwrap_or(0.0);
                flds.push(("Score", format!("{score:.1} / {pts}"), Style::default().fg(SUCCESS)));
            }
            if let Some(grade) = sub.grade.as_deref() {
                if sub.score.is_none() {
                    flds.push(("Grade", grade.to_string(), Style::default().fg(SUCCESS)));
                }
            }
            if let Some(submitted) = sub.submitted_at {
                flds.push((
                    "Submitted",
                    submitted.format("%B %d, %Y at %H:%M").to_string(),
                    value_style,
                ));
            }
            if let Some(graded) = sub.graded_at {
                flds.push((
                    "Graded",
                    graded.format("%B %d, %Y").to_string(),
                    value_style,
                ));
            }
            if let Some(attempt) = sub.attempt {
                flds.push(("Attempt", attempt.to_string(), value_style));
            }
            if let Some(late) = sub.late {
                let (text, color) = if late {
                    ("Yes", DANGER)
                } else {
                    ("No", value_style.fg.unwrap_or(TEXT))
                };
                flds.push(("Late", text.to_string(), Style::default().fg(color)));
            }
            if let Some(missing) = sub.missing {
                if missing {
                    flds.push(("Missing", "Yes".to_string(), Style::default().fg(DANGER)));
                }
            }
        }

        flds
    };

    for (label, value, style) in &fields {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<10}", label), label_style),
            Span::styled(value.as_str(), *style),
        ]));
    }

    if let Some(ref desc) = assignment.description {
        let stripped = strip_html(desc);
        if !stripped.trim().is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ── Description ──────────────────────────────",
                Style::default().fg(TEXT_MUTED),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", stripped.trim()),
                Style::default().fg(TEXT_DIM),
            )));
        }
    }

    if let Some(ref url) = assignment.html_url {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ── Link ─────────────────────────────────────",
            Style::default().fg(TEXT_MUTED),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {url}"),
            Style::default().fg(INFO),
        )));
    }

    let detail = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(detail_block);

    f.render_widget(detail, area);
}

// ─── Calendar list (shared by render_schedule_calendar) ──────────────────────

fn render_calendar_list(f: &mut Frame, app: &mut App, area: Rect) {
    let local_now = Local::now();
    let today = local_now.date_naive();
    let focal_id = app.focal_assignment_id;

    // Group: (iso_year, iso_week) → NaiveDate → Vec<(original_idx, &CalendarItem)>
    let mut by_week: BTreeMap<(i32, u32), BTreeMap<NaiveDate, Vec<(usize, &CalendarItem)>>> =
        BTreeMap::new();
    let mut undated: Vec<(usize, &CalendarItem)> = Vec::new();

    for (i, item) in app.calendar_items.iter().enumerate() {
        if let Some(dt) = item.start_at {
            let date = dt.with_timezone(&Local).date_naive();
            let iso = date.iso_week();
            by_week
                .entry((iso.year(), iso.week()))
                .or_default()
                .entry(date)
                .or_default()
                .push((i, item));
        } else {
            undated.push((i, item));
        }
    }

    let mut list_items: Vec<ListItem> = Vec::new();
    let mut selected_item_idx = 0usize;

    for ((_, week_num), days) in &by_week {
        let first_date = *days.keys().next().unwrap();
        let last_date = *days.keys().last().unwrap();

        // Which weekdays (0=Mon … 6=Sun) have items
        let active_days: std::collections::HashSet<u32> =
            days.keys().map(|d| d.weekday().num_days_from_monday()).collect();

        // Week date-range label
        let week_range = if first_date.month() == last_date.month() {
            format!("{} – {}", first_date.format("%b %d"), last_date.format("%d"))
        } else {
            format!(
                "{} – {}",
                first_date.format("%b %d"),
                last_date.format("%b %d")
            )
        };

        // Day-dot strip: M T W T F S S  with ● for active days
        let day_initials = ["M", "T", "W", "T", "F", "S", "S"];
        let mut week_spans: Vec<Span> = vec![
            Span::styled(
                format!(" ◈  Wk {week_num:<2}  "),
                Style::default()
                    .fg(AMBER_SOFT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{week_range}   "),
                Style::default().fg(TEXT_DIM),
            ),
        ];
        for (d, &init) in day_initials.iter().enumerate() {
            let has = active_days.contains(&(d as u32));
            week_spans.push(Span::styled(
                format!("{}{} ", init, if has { "●" } else { " " }),
                Style::default().fg(if has { CAUTION } else { TEXT_MUTED }),
            ));
        }
        list_items.push(ListItem::new(Line::from(week_spans)));

        // Items within the week, grouped by day
        for (date, day_items) in days {
            let is_today = *date == today;
            let is_past = *date < today;

            let (day_color, day_label) = if is_today {
                (
                    AMBER,
                    format!("  ◈ Today · {} ", date.format("%A, %b %d")),
                )
            } else if is_past {
                (
                    TEXT_MUTED,
                    format!("  ─ {} ", date.format("%A, %b %d")),
                )
            } else {
                (
                    TEXT_DIM,
                    format!("  ─ {} ", date.format("%A, %b %d")),
                )
            };

            list_items.push(ListItem::new(Line::from(Span::styled(
                day_label,
                Style::default()
                    .fg(day_color)
                    .add_modifier(if is_today { Modifier::BOLD } else { Modifier::empty() }),
            ))));

            for (item_idx, item) in day_items {
                let is_selected = *item_idx == app.calendar_list_state.selected;
                let is_focal =
                    item.assignment_id.is_some() && item.assignment_id == focal_id;

                if is_selected {
                    selected_item_idx = list_items.len();
                }

                let bg = if is_selected {
                    SEL_BG
                } else if is_focal {
                    FOCAL_BG
                } else {
                    Color::Reset
                };

                let (marker, marker_fg) = if is_selected {
                    ("▶", AMBER)
                } else if is_focal {
                    ("»", FOCAL)
                } else {
                    (" ", TEXT_MUTED)
                };

                let time = item
                    .start_at
                    .map(|d| d.with_timezone(&Local).format("%H:%M").to_string())
                    .unwrap_or_else(|| "─────".into());

                let (type_icon, type_color) = if item.item_type == "assignment" {
                    ("◆", if is_focal { FOCAL } else { DANGER })
                } else {
                    ("◇", INFO)
                };

                let title_style = Style::default()
                    .fg(if is_past { TEXT_DIM } else { TEXT })
                    .bg(bg)
                    .add_modifier(
                        if is_selected || is_focal {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        },
                    );

                let status_span = match &item.status {
                    Some(s) => {
                        let sc = if s.starts_with("Missing") {
                            DANGER
                        } else if s.starts_with("Past due") {
                            CAUTION
                        } else if s.starts_with("Submitted") {
                            INFO
                        } else {
                            SUCCESS
                        };
                        Span::styled(
                            format!("  [{}]", s),
                            Style::default().fg(sc).bg(bg),
                        )
                    }
                    None => Span::styled("", Style::default().bg(bg)),
                };

                list_items.push(ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" {} ", marker),
                            Style::default().fg(marker_fg).bg(bg),
                        ),
                        Span::styled(
                            format!("{time}  "),
                            Style::default().fg(TEXT_DIM).bg(bg),
                        ),
                        Span::styled(
                            format!("{type_icon}  "),
                            Style::default().fg(type_color).bg(bg),
                        ),
                        Span::styled(item.title.clone(), title_style),
                    ]),
                    Line::from(vec![
                        Span::styled("           ", Style::default().bg(bg)),
                        Span::styled(
                            item.course_name.as_deref().unwrap_or("").to_string(),
                            Style::default().fg(TEXT_MUTED).bg(bg),
                        ),
                        status_span,
                    ]),
                ]));
            }
        }
    }

    // Undated items
    if !undated.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            "  ─ No date ─────────────────────────",
            Style::default().fg(TEXT_MUTED),
        ))));
        for (item_idx, item) in &undated {
            let is_selected = *item_idx == app.calendar_list_state.selected;
            if is_selected {
                selected_item_idx = list_items.len();
            }
            let bg = if is_selected { SEL_BG } else { Color::Reset };
            let (marker, marker_fg) =
                if is_selected { ("▶", AMBER) } else { (" ", TEXT_MUTED) };
            let (type_icon, type_color) = if item.item_type == "assignment" {
                ("◆", DANGER)
            } else {
                ("◇", INFO)
            };
            list_items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", marker),
                        Style::default().fg(marker_fg).bg(bg),
                    ),
                    Span::styled(
                        format!("{type_icon}  "),
                        Style::default().fg(type_color).bg(bg),
                    ),
                    Span::styled(
                        item.title.clone(),
                        Style::default()
                            .fg(TEXT)
                            .bg(bg)
                            .add_modifier(if is_selected {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("     ", Style::default().bg(bg)),
                    Span::styled(
                        item.course_name.as_deref().unwrap_or("").to_string(),
                        Style::default().fg(TEXT_MUTED).bg(bg),
                    ),
                ]),
            ]));
        }
    }

    if list_items.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            "  ○  No calendar entries found.",
            Style::default().fg(TEXT_DIM),
        ))));
    }

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(format!(" Schedule [Calendar] ({})   v: list   Enter: submit ", app.calendar_items.len()))
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    app.calendar_list_state
        .inner
        .select(Some(selected_item_idx));
    f.render_stateful_widget(list, area, &mut app.calendar_list_state.inner);
}

/// Render a lightweight detail view for a pure calendar event (no assignment backing).
fn render_calendar_event_detail<'a>(
    f: &mut Frame,
    area: Rect,
    detail_block: Block<'a>,
    item: &CalendarItem,
) {
    let now = Utc::now();
    let today = now.date_naive();
    let label_style = Style::default().fg(AMBER_SOFT);
    let value_style = Style::default().fg(TEXT);

    let (type_icon, type_color, type_str) = if item.item_type == "assignment" {
        ("◆", DANGER, "Assignment")
    } else {
        ("◇", INFO, "Event")
    };

    let (date_line, time_line) = if let Some(dt) = item.start_at {
        let local = dt.with_timezone(&Local);
        let date = local.date_naive();
        let date_str = if date == today {
            format!("{} (Today)", local.format("%A, %B %d, %Y"))
        } else {
            local.format("%A, %B %d, %Y").to_string()
        };
        (date_str, local.format("%H:%M").to_string())
    } else {
        ("No date".into(), "─".into())
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {type_icon} "), Style::default().fg(type_color)),
            Span::styled(
                item.title.clone(),
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ── ", Style::default().fg(TEXT_MUTED)),
            Span::styled(type_str, Style::default().fg(type_color)),
            Span::styled(
                " ─────────────────────────────",
                Style::default().fg(TEXT_MUTED),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Date      ", label_style),
            Span::styled(date_line, value_style),
        ]),
        Line::from(vec![
            Span::styled("  Time      ", label_style),
            Span::styled(time_line, value_style),
        ]),
    ];

    if let Some(ref course) = item.course_name {
        lines.push(Line::from(vec![
            Span::styled("  Course    ", label_style),
            Span::styled(course.clone(), value_style),
        ]));
    }

    if let Some(ref status) = item.status {
        let sc = if status.starts_with("Missing") {
            DANGER
        } else if status.starts_with("Past due") {
            CAUTION
        } else if status.starts_with("Submitted") {
            INFO
        } else {
            SUCCESS
        };
        lines.push(Line::from(vec![
            Span::styled("  Status    ", label_style),
            Span::styled(status.clone(), Style::default().fg(sc)),
        ]));
    }

    if let Some(dt) = item.start_at {
        let (timer_text, timer_color) = countdown_timer(dt);
        let timer_label = if item.item_type == "assignment" {
            "  Due in    "
        } else {
            "  In        "
        };
        lines.push(Line::from(vec![
            Span::styled(timer_label, label_style),
            Span::styled(timer_text, Style::default().fg(timer_color)),
        ]));
    }

    let detail = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(detail_block);

    f.render_widget(detail, area);
}

// ─── Announcements ───────────────────────────────────────────────────────────

fn render_announcements(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    let items: Vec<ListItem> = app
        .announcements
        .iter()
        .enumerate()
        .map(|(i, ann)| {
            let title = ann.title.as_deref().unwrap_or("Untitled");
            let author = ann.user_name.as_deref().unwrap_or("Unknown");
            let date = ann
                .posted_at
                .map(|d| d.format("%b %d").to_string())
                .unwrap_or_default();

            let is_unread = ann.read_state.as_deref() == Some("unread");
            let is_selected = i == app.announcement_list_state.selected;
            let bg = if is_selected { SEL_BG } else { Color::Reset };

            let (marker, marker_fg) = if is_selected { ("▶", AMBER) } else { (" ", TEXT_MUTED) };

            let title_style = if is_unread {
                Style::default().fg(TEXT).bg(bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT_DIM).bg(bg)
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", marker), Style::default().fg(marker_fg).bg(bg)),
                    if is_unread {
                        Span::styled("● ", Style::default().fg(DANGER).bg(bg))
                    } else {
                        Span::styled("  ", Style::default().bg(bg))
                    },
                    Span::styled(title, title_style),
                ]),
                Line::from(vec![
                    Span::styled("      ", Style::default().bg(bg)),
                    Span::styled(author, Style::default().fg(TEXT_MUTED).bg(bg)),
                    Span::styled(format!("  {date}"), Style::default().fg(TEXT_MUTED).bg(bg)),
                ]),
            ])
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(format!(" Announcements ({}) ", app.announcements.len()))
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    app.announcement_list_state
        .inner
        .select(Some(app.announcement_list_state.selected));
    f.render_stateful_widget(list, chunks[0], &mut app.announcement_list_state.inner);

    let detail = if let Some(ann) = app.announcements.get(app.announcement_list_state.selected) {
        let title = ann.title.as_deref().unwrap_or("Untitled");
        let author = ann.user_name.as_deref().unwrap_or("Unknown");
        let date = ann
            .posted_at
            .map(|d| d.format("%B %d, %Y at %H:%M").to_string())
            .unwrap_or_default();
        let body = strip_html(ann.message.as_deref().unwrap_or("(no content)"));

        Paragraph::new(vec![
            Line::from(Span::styled(
                title,
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("by {author}  ─  {date}"),
                Style::default().fg(TEXT_MUTED),
            )),
            Line::from(""),
            Line::from(Span::styled(body, Style::default().fg(TEXT_DIM))),
        ])
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(TEXT_MUTED))
                .title(" Detail ")
                .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        )
    } else {
        Paragraph::new(Line::from(Span::styled(
            "  Select an announcement to view details.",
            Style::default().fg(TEXT_DIM),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(TEXT_MUTED))
                .title(" Detail ")
                .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        )
    };

    f.render_widget(detail, chunks[1]);
}

// ─── Utilities ───────────────────────────────────────────────────────────────

fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
}
