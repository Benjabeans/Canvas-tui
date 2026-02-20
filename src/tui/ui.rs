use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use super::{App, AssignmentSort, Tab};
use crate::models::Assignment;
use chrono::{Local, Utc};

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
        Tab::Assignments => render_assignments(f, app, chunks[1]),
        Tab::Calendar => render_calendar(f, app, chunks[1]),
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

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", indicator),
            Style::default().fg(ind_color).bg(HDR_BG),
        ),
        Span::styled(
            app.status_message.as_str(),
            Style::default().fg(TEXT).bg(HDR_BG),
        ),
        Span::styled(
            format!(
                "   │   q quit   Tab switch   j/k nav   s sort   t today   r refresh{}  ",
                sync_hint
            ),
            Style::default().fg(TEXT_MUTED).bg(HDR_BG),
        ),
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

fn render_assignments(f: &mut Frame, app: &mut App, area: Rect) {
    let sort_label = app.assignment_sort.label();
    let block_title = format!(" Assignments   s: {} ", sort_label);

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
}

fn render_assignments_grouped(f: &mut Frame, app: &mut App, area: Rect, block_title: &str) {
    let focal_id = app.focal_assignment_id;
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_idx = 0usize;
    let mut selected_item_idx = 0usize;

    for (course_name, assignments) in &app.assignments {
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

    let name = assignment.name.as_deref().unwrap_or("Unnamed");
    let now = Utc::now();
    let today = now.date_naive();

    // Due date formatting
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

    // Helper to add a label: value row
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
                f.push(("Score", format!("{score:.1} / {pts}"), Style::default().fg(SUCCESS)));
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

    // Description section
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

    // Link section
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

// ─── Calendar ────────────────────────────────────────────────────────────────

fn render_calendar(f: &mut Frame, app: &mut App, area: Rect) {
    let focal_id = app.focal_assignment_id;
    let mut items: Vec<ListItem> = Vec::new();
    let mut current_date = String::new();
    let mut selected_item_idx = 0usize;

    for (i, entry) in app.calendar_items.iter().enumerate() {
        let date_str = entry
            .start_at
            .map(|d| d.format("%A, %b %d").to_string())
            .unwrap_or_else(|| "Unknown date".into());

        if date_str != current_date {
            current_date = date_str.clone();
            items.push(ListItem::new(Line::from(vec![
                Span::styled(" ▸  ", Style::default().fg(CAUTION)),
                Span::styled(
                    date_str,
                    Style::default().fg(CAUTION).add_modifier(Modifier::BOLD),
                ),
            ])));
        }

        let time = entry
            .start_at
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_else(|| "─────".into());

        let is_selected = i == app.calendar_list_state.selected;
        let is_focal = entry.assignment_id.is_some() && entry.assignment_id == focal_id;

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

        let type_color = match entry.item_type {
            "assignment" => if is_focal { FOCAL } else { DANGER },
            _ => INFO,
        };

        let type_badge = match entry.item_type {
            "assignment" => "assign",
            _ => "event ",
        };

        let title_style = Style::default().fg(TEXT).bg(bg).add_modifier(
            if is_focal && !is_selected { Modifier::BOLD } else { Modifier::empty() },
        );

        let mut spans = vec![
            Span::styled(format!(" {} ", marker), Style::default().fg(marker_fg).bg(bg)),
            Span::styled(format!("{time}  "), Style::default().fg(TEXT_DIM).bg(bg)),
            Span::styled(
                format!("[{type_badge}]  "),
                Style::default().fg(type_color).bg(bg),
            ),
            Span::styled(entry.title.clone(), title_style),
        ];

        if let Some(ref course) = entry.course_name {
            spans.push(Span::styled(
                format!("  ─  {}", course),
                Style::default().fg(TEXT_MUTED).bg(bg),
            ));
        }

        if let Some(ref status) = entry.status {
            let status_color = if status.starts_with("Missing") {
                DANGER
            } else if status.starts_with("Past due") {
                CAUTION
            } else if status.starts_with("Submitted") {
                INFO
            } else {
                SUCCESS
            };
            spans.push(Span::styled(
                format!("   [{}]", status),
                Style::default().fg(status_color).bg(bg),
            ));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  ○  No calendar entries found.",
            Style::default().fg(TEXT_DIM),
        ))));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(TEXT_MUTED))
            .title(format!(" Calendar ({}) ", app.calendar_items.len()))
            .title_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    );

    app.calendar_list_state.inner.select(Some(selected_item_idx));
    f.render_stateful_widget(list, area, &mut app.calendar_list_state.inner);
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
