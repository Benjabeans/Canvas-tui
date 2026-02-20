use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

use super::App;

pub fn poll_event(timeout: Duration) -> anyhow::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // ── Course filter popup intercepts all keys while open ────────────
    if app.show_course_filter {
        handle_filter_key(app, code);
        return;
    }

    match (code, modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.running = false;
            return;
        }
        (KeyCode::Tab, _) | (KeyCode::Right, KeyModifiers::SHIFT) => {
            app.active_tab = app.active_tab.next();
            return;
        }
        (KeyCode::BackTab, _) | (KeyCode::Left, KeyModifiers::SHIFT) => {
            app.active_tab = app.active_tab.prev();
            return;
        }
        (KeyCode::Char('1'), _) => {
            app.active_tab = super::Tab::Dashboard;
            return;
        }
        (KeyCode::Char('2'), _) => {
            app.active_tab = super::Tab::Courses;
            return;
        }
        (KeyCode::Char('3'), _) => {
            app.active_tab = super::Tab::Assignments;
            return;
        }
        (KeyCode::Char('4'), _) => {
            app.active_tab = super::Tab::Calendar;
            return;
        }
        (KeyCode::Char('5'), _) => {
            app.active_tab = super::Tab::Announcements;
            return;
        }
        _ => {}
    }

    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            app.active_list_state_mut().select_next();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.active_list_state_mut().select_prev();
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.active_list_state_mut().selected = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            let ls = app.active_list_state_mut();
            if ls.len > 0 {
                ls.selected = ls.len - 1;
            }
        }
        KeyCode::Char('s') if app.active_tab == super::Tab::Assignments => {
            app.assignment_sort = app.assignment_sort.next();
            app.assignment_list_state.selected = 0;
        }
        KeyCode::Char('f') if app.active_tab == super::Tab::Assignments => {
            let count = app.assignment_course_names().len();
            app.filter_list_state.set_len(count);
            app.filter_list_state.selected = 0;
            app.show_course_filter = true;
        }
        KeyCode::Char('t') => {
            app.jump_to_today_active();
        }
        KeyCode::Char('r') if !app.loading => {
            app.needs_refresh = true;
        }
        _ => {}
    }
}

fn handle_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            app.filter_list_state.select_next();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.filter_list_state.select_prev();
        }
        KeyCode::Char(' ') => {
            let names = app.assignment_course_names();
            if let Some(name) = names.get(app.filter_list_state.selected) {
                let name = name.to_string();
                app.toggle_course_filter(&name);
            }
        }
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('f') => {
            app.show_course_filter = false;
        }
        _ => {}
    }
}
