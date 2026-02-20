use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

use super::{App, AssignmentSort, Tab};
use crate::models::Assignment;
use chrono::{Local, Utc};

const ACCENT: Color = Color::Cyan;
const HEADER_BG: Color = Color::DarkGray;
const SELECTED_BG: Color = Color::Rgb(40, 40, 60);
/// Background for the focal assignment (next actionable item).
const FOCAL_BG: Color = Color::Rgb(60, 42, 0);
/// Foreground accent for the focal marker/text.
const FOCAL: Color = Color::Rgb(255, 185, 50);
const DIM: Color = Color::DarkGray;
const GOOD: Color = Color::Green;
const WARN: Color = Color::Yellow;
const BAD: Color = Color::Red;

// ─── Main render ────────────────────────────────────────────────────────────

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

// ─── Tab Bar ────────────────────────────────────────────────────────────────

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            Line::from(vec![
                Span::styled(format!(" {} ", i + 1), Style::default().fg(DIM)),
                Span::styled(format!("{} ", tab.title()), Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let selected = Tab::ALL
        .iter()
        .position(|t| *t == app.active_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .title(" Canvas TUI ")
                .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        )
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );

    f.render_widget(tabs, area);
}

// ─── Clock ──────────────────────────────────────────────────────────────────

fn render_clock(f: &mut Frame, tab_area: Rect) {
    let time_str = format!(" {} ", Local::now().format("%a %b %d  %H:%M:%S"));
    let clock_width = time_str.len() as u16;
    let clock_area = Rect {
        x: tab_area.right().saturating_sub(clock_width),
        y: tab_area.y,
        width: clock_width.min(tab_area.width),
        height: 1,
    };
    f.render_widget(
        Paragraph::new(time_str).style(Style::default().fg(ACCENT)),
        clock_area,
    );
}

// ─── Status Bar ─────────────────────────────────────────────────────────────

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let sync_hint = app
        .cached_at
        .map(|t| {
            format!(
                "  synced {}",
                t.with_timezone(&Local).format("%b %d %H:%M")
            )
        })
        .unwrap_or_default();

    let status = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            &app.status_message,
            Style::default().fg(if app.loading { WARN } else { Color::White }),
        ),
        Span::styled(
            format!(
                "  q:quit  Tab:switch  j/k:nav  s:sort  t:today  r:refresh{}  ",
                sync_hint
            ),
            Style::default().fg(DIM),
        ),
    ]))
    .style(Style::default().bg(HEADER_BG));

    f.render_widget(status, area);
}

// ─── Dashboard ──────────────────────────────────────────────────────────────

fn render_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let user_name = app
        .user
        .as_ref()
        .and_then(|u| u.name.clone())
        .unwrap_or_else(|| "Student".into());

    let upcoming_count = app.calendar_events.len();
    let unread_count = app
        .announcements
        .iter()
        .filter(|a| a.read_state.as_deref() == Some("unread"))
        .count();

    let summary = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("  Welcome, {}!", user_name),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} courses", app.courses.len()), Style::default().fg(ACCENT)),
            Span::styled("  |  ", Style::default().fg(DIM)),
            Span::styled(format!("{} upcoming events", upcoming_count), Style::default().fg(WARN)),
            Span::styled("  |  ", Style::default().fg(DIM)),
            Span::styled(
                format!("{} unread announcements", unread_count),
                Style::default().fg(if unread_count > 0 { BAD } else { GOOD }),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Overview ")
            .title_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(summary, chunks[0]);

    if app.grades.is_empty() {
        let msg = Paragraph::new(
            "  No grade data available (you may not be enrolled as a student)",
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Grades ")
                .title_style(Style::default().fg(ACCENT)),
        );
        f.render_widget(msg, chunks[1]);
    } else {
        let header =
            Row::new(vec!["Course", "Score", "Grade", "Final Score", "Final Grade"])
                .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
                .bottom_margin(1);

        let rows: Vec<Row> = app
            .grades
            .iter()
            .map(|g| {
                let score_style = match g.current_score {
                    Some(s) if s >= 90.0 => Style::default().fg(GOOD),
                    Some(s) if s >= 70.0 => Style::default().fg(WARN),
                    Some(_) => Style::default().fg(BAD),
                    None => Style::default().fg(DIM),
                };
                Row::new(vec![
                    g.course_name.clone(),
                    g.current_score
                        .map(|s| format!("{:.1}%", s))
                        .unwrap_or_else(|| "-".into()),
                    g.current_grade.clone().unwrap_or_else(|| "-".into()),
                    g.final_score
                        .map(|s| format!("{:.1}%", s))
                        .unwrap_or_else(|| "-".into()),
                    g.final_grade.clone().unwrap_or_else(|| "-".into()),
                ])
                .style(score_style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
        .header(header)
        .row_highlight_style(Style::default().bg(SELECTED_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Grades ")
                .title_style(Style::default().fg(ACCENT)),
        );

        app.grades_table_state.select(Some(app.course_list_state.selected));
        f.render_stateful_widget(table, chunks[1], &mut app.grades_table_state);
    }
}

// ─── Courses ────────────────────────────────────────────────────────────────

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
            let marker = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                Style::default().bg(SELECTED_BG).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(ACCENT)),
                Span::styled(name, style.add_modifier(Modifier::BOLD)),
                Span::styled(format!("  ({code})"), Style::default().fg(DIM)),
                Span::styled(format!("  {students}"), Style::default().fg(DIM)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Courses ({}) ", app.courses.len()))
            .title_style(Style::default().fg(ACCENT)),
    );

    app.course_list_state.inner.select(Some(app.course_list_state.selected));
    f.render_stateful_widget(list, area, &mut app.course_list_state.inner);
}

// ─── Assignments ────────────────────────────────────────────────────────────

fn assignment_status(a: &Assignment) -> (String, Color) {
    let now = Utc::now();
    if let Some(ref sub) = a.submission {
        match sub.workflow_state.as_deref() {
            Some("graded") => {
                let grade = sub
                    .score
                    .map(|s| format!("{s:.1}/{}", a.points_possible.unwrap_or(0.0)))
                    .unwrap_or_else(|| "Graded".into());
                (grade, GOOD)
            }
            Some("submitted") => ("Submitted".into(), ACCENT),
            _ => {
                if a.due_at.map_or(false, |d| d < now) {
                    if sub.missing.unwrap_or(false) {
                        ("Missing!".into(), BAD)
                    } else {
                        ("Past due".into(), WARN)
                    }
                } else {
                    ("Not submitted".into(), DIM)
                }
            }
        }
    } else if a.due_at.map_or(false, |d| d < now) {
        ("Past due".into(), WARN)
    } else {
        ("-".into(), DIM)
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
    let block_title = format!(" Assignments  [s: {}] ", sort_label);

    if app.assignment_sort == AssignmentSort::Course {
        render_assignments_grouped(f, app, area, &block_title);
    } else {
        render_assignments_flat(f, app, area, &block_title);
    }
}

fn render_assignments_grouped(f: &mut Frame, app: &mut App, area: Rect, block_title: &str) {
    let focal_id = app.focal_assignment_id;
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_idx = 0usize;
    let mut selected_item_idx = 0usize;

    for (course_name, assignments) in &app.assignments {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("── {} ──", course_name),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))));

        for assignment in assignments {
            let is_selected = flat_idx == app.assignment_list_state.selected;
            let is_focal = Some(assignment.id) == focal_id;

            let (marker, marker_color) = if is_selected {
                ("> ", ACCENT)
            } else if is_focal {
                ("» ", FOCAL)
            } else {
                ("  ", ACCENT)
            };

            let bg = if is_selected {
                SELECTED_BG
            } else if is_focal {
                FOCAL_BG
            } else {
                Color::Reset
            };

            if is_selected {
                selected_item_idx = items.len();
            }

            let name = assignment.name.as_deref().unwrap_or("Unnamed");
            let due = assignment
                .due_at
                .map(|d| d.format("%b %d, %H:%M").to_string())
                .unwrap_or_else(|| "No due date".into());
            let points = assignment
                .points_possible
                .map(|p| format!("{p} pts"))
                .unwrap_or_default();
            let (status, status_color) = assignment_status(assignment);

            let name_style = Style::default().fg(Color::White).bg(bg).add_modifier(
                if is_focal && !is_selected { Modifier::BOLD } else { Modifier::empty() },
            );

            items.push(ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(marker_color)),
                Span::styled(format!("{name:<40}"), name_style),
                Span::styled(format!(" {due:<18}"), Style::default().fg(DIM).bg(bg)),
                Span::styled(format!(" {points:<10}"), Style::default().fg(DIM).bg(bg)),
                Span::styled(format!(" {status}"), Style::default().fg(status_color).bg(bg)),
            ])));

            flat_idx += 1;
        }
    }

    if items.is_empty() {
        items.push(ListItem::new("  No assignments found."));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(block_title.to_string())
            .title_style(Style::default().fg(ACCENT)),
    );

    app.assignment_list_state.inner.select(Some(selected_item_idx));
    f.render_stateful_widget(list, area, &mut app.assignment_list_state.inner);
}

fn render_assignments_flat(f: &mut Frame, app: &mut App, area: Rect, block_title: &str) {
    let focal_id = app.focal_assignment_id;

    let mut flat: Vec<(&str, &Assignment)> = app
        .assignments
        .iter()
        .flat_map(|(course, assignments)| {
            assignments.iter().map(move |a| (course.as_str(), a))
        })
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

        let (marker, marker_color) = if is_selected {
            ("> ", ACCENT)
        } else if is_focal {
            ("» ", FOCAL)
        } else {
            ("  ", ACCENT)
        };

        let bg = if is_selected {
            SELECTED_BG
        } else if is_focal {
            FOCAL_BG
        } else {
            Color::Reset
        };

        let name = assignment.name.as_deref().unwrap_or("Unnamed");
        let due = assignment
            .due_at
            .map(|d| d.format("%b %d, %H:%M").to_string())
            .unwrap_or_else(|| "No due date".into());
        let points = assignment
            .points_possible
            .map(|p| format!("{p} pts"))
            .unwrap_or_default();
        let (status, status_color) = assignment_status(assignment);

        let name_style = Style::default().fg(Color::White).bg(bg).add_modifier(
            if is_focal && !is_selected { Modifier::BOLD } else { Modifier::empty() },
        );

        items.push(ListItem::new(Line::from(vec![
            Span::styled(marker, Style::default().fg(marker_color)),
            Span::styled(format!("{name:<36}"), name_style),
            Span::styled(format!(" {:<20}", course_name), Style::default().fg(DIM).bg(bg)),
            Span::styled(format!(" {due:<18}"), Style::default().fg(DIM).bg(bg)),
            Span::styled(format!(" {points:<10}"), Style::default().fg(DIM).bg(bg)),
            Span::styled(format!(" {status}"), Style::default().fg(status_color).bg(bg)),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new("  No assignments found."));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(block_title.to_string())
            .title_style(Style::default().fg(ACCENT)),
    );

    app.assignment_list_state
        .inner
        .select(Some(app.assignment_list_state.selected));
    f.render_stateful_widget(list, area, &mut app.assignment_list_state.inner);
}

// ─── Calendar ───────────────────────────────────────────────────────────────

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
            items.push(ListItem::new(Line::from(Span::styled(
                format!("── {} ──", date_str),
                Style::default().fg(WARN).add_modifier(Modifier::BOLD),
            ))));
        }

        let time = entry
            .start_at
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_else(|| "     ".into());

        let is_selected = i == app.calendar_list_state.selected;
        let is_focal = entry.assignment_id.is_some() && entry.assignment_id == focal_id;

        if is_selected {
            selected_item_idx = items.len();
        }

        let type_color = match entry.item_type {
            "assignment" => {
                if is_focal { FOCAL } else { BAD }
            }
            _ => ACCENT,
        };

        let bg = if is_selected {
            SELECTED_BG
        } else if is_focal {
            FOCAL_BG
        } else {
            Color::Reset
        };

        let (marker, marker_color) = if is_selected {
            ("> ", ACCENT)
        } else if is_focal {
            ("» ", FOCAL)
        } else {
            ("  ", ACCENT)
        };

        let title_style = Style::default().fg(Color::White).bg(bg).add_modifier(
            if is_focal && !is_selected { Modifier::BOLD } else { Modifier::empty() },
        );

        let mut spans = vec![
            Span::styled(marker, Style::default().fg(marker_color)),
            Span::styled(format!("{time}  "), Style::default().fg(DIM).bg(bg)),
            Span::styled(format!("[{}] ", entry.item_type), Style::default().fg(type_color).bg(bg)),
            Span::styled(entry.title.clone(), title_style),
        ];

        if let Some(ref course) = entry.course_name {
            spans.push(Span::styled(
                format!("  ({})", course),
                Style::default().fg(DIM).bg(bg),
            ));
        }

        if let Some(ref status) = entry.status {
            let status_color = if status.starts_with("Missing") {
                BAD
            } else if status.starts_with("Past due") {
                WARN
            } else if status.starts_with("Submitted") {
                ACCENT
            } else {
                GOOD
            };
            spans.push(Span::styled(
                format!("  [{}]", status),
                Style::default().fg(status_color).bg(bg),
            ));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    if items.is_empty() {
        items.push(ListItem::new("  No calendar entries found."));
    }

    let title = format!(" Calendar ({}) ", app.calendar_items.len());
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(ACCENT)),
    );

    app.calendar_list_state.inner.select(Some(selected_item_idx));
    f.render_stateful_widget(list, area, &mut app.calendar_list_state.inner);
}

// ─── Announcements ──────────────────────────────────────────────────────────

fn render_announcements(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
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
            let marker = if is_selected { "> " } else { "  " };

            let title_style = if is_unread {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DIM)
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(marker, Style::default().fg(ACCENT)),
                    Span::styled(
                        title,
                        if is_selected { title_style.bg(SELECTED_BG) } else { title_style },
                    ),
                    if is_unread {
                        Span::styled(" *", Style::default().fg(BAD))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(author, Style::default().fg(DIM)),
                    Span::styled(format!("  {date}"), Style::default().fg(DIM)),
                ]),
            ])
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Announcements ({}) ", app.announcements.len()))
            .title_style(Style::default().fg(ACCENT)),
    );

    app.announcement_list_state
        .inner
        .select(Some(app.announcement_list_state.selected));
    f.render_stateful_widget(list, chunks[0], &mut app.announcement_list_state.inner);

    let detail = if let Some(ann) = app
        .announcements
        .get(app.announcement_list_state.selected)
    {
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
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("By {author} - {date}"),
                Style::default().fg(DIM),
            )),
            Line::from(""),
            Line::from(Span::raw(body)),
        ])
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detail ")
                .title_style(Style::default().fg(ACCENT)),
        )
    } else {
        Paragraph::new("  Select an announcement to view details.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detail ")
                .title_style(Style::default().fg(ACCENT)),
        )
    };
    f.render_widget(detail, chunks[1]);
}

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
