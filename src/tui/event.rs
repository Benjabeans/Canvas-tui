use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

use super::{App, SubmissionKind, SubmissionState, UnifiedViewMode};

pub fn poll_event(timeout: Duration) -> anyhow::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // ── Submission modal intercepts everything while open ─────────────
    if !app.submission_state.is_hidden() {
        handle_submission_key(app, code);
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
        KeyCode::Char('v') if app.active_tab == super::Tab::Assignments => {
            app.unified_view_mode = app.unified_view_mode.toggle();
            // Jump to today when switching into calendar view.
            match app.unified_view_mode {
                UnifiedViewMode::CalendarView => {
                    let idx = app.find_today_calendar_idx();
                    app.calendar_list_state.selected = idx;
                }
                UnifiedViewMode::ListView => {
                    app.assignment_list_state.selected = 0;
                }
            }
        }
        KeyCode::Char('s')
            if app.active_tab == super::Tab::Assignments
                && app.unified_view_mode == UnifiedViewMode::ListView =>
        {
            app.assignment_sort = app.assignment_sort.next();
            app.assignment_list_state.selected = 0;
        }
        KeyCode::Char('f')
            if app.active_tab == super::Tab::Assignments
                && app.unified_view_mode == UnifiedViewMode::ListView =>
        {
            let count = app.assignment_course_names().len();
            app.filter_list_state.set_len(count);
            app.filter_list_state.selected = 0;
            app.show_course_filter = true;
        }
        // Open submission modal for the selected item (works in both view modes).
        KeyCode::Enter if app.active_tab == super::Tab::Assignments => {
            app.open_submission_modal();
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

fn handle_submission_key(app: &mut App, code: KeyCode) {
    // Clone the current state so we can pattern-match while mutating app.
    let state = std::mem::replace(&mut app.submission_state, SubmissionState::Hidden);

    match state {
        // ── TypePicker ────────────────────────────────────────────────
        SubmissionState::TypePicker => match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.submission_cursor > 0 {
                    app.submission_cursor -= 1;
                }
                app.submission_state = SubmissionState::TypePicker;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.submission_cursor + 1 < app.submission_supported_kinds.len() {
                    app.submission_cursor += 1;
                }
                app.submission_state = SubmissionState::TypePicker;
            }
            KeyCode::Enter => {
                let kind = app
                    .submission_supported_kinds
                    .get(app.submission_cursor)
                    .cloned();
                match kind {
                    Some(SubmissionKind::TextEntry) => {
                        app.submission_kind = Some(SubmissionKind::TextEntry);
                        app.launch_editor = true;
                        // State stays Hidden until the editor returns; main.rs
                        // sets it to TextPreview or back to TypePicker.
                        app.submission_state = SubmissionState::Hidden;
                    }
                    Some(SubmissionKind::Url) => {
                        app.submission_kind = Some(SubmissionKind::Url);
                        app.submission_input.clear();
                        app.submission_state = SubmissionState::UrlInput;
                    }
                    Some(SubmissionKind::FileUpload) => {
                        app.submission_kind = Some(SubmissionKind::FileUpload);
                        app.submission_input.clear();
                        app.submission_state = SubmissionState::FileInput;
                    }
                    None => {
                        app.submission_state = SubmissionState::TypePicker;
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                app.submission_state = SubmissionState::Hidden;
            }
            _ => {
                app.submission_state = SubmissionState::TypePicker;
            }
        },

        // ── UrlInput / FileInput (same controls) ──────────────────────
        SubmissionState::UrlInput | SubmissionState::FileInput => {
            let next_state = if matches!(state, SubmissionState::UrlInput) {
                SubmissionState::UrlInput
            } else {
                SubmissionState::FileInput
            };
            match code {
                KeyCode::Char(c) => {
                    app.submission_input.push(c);
                    app.submission_state = next_state;
                }
                KeyCode::Backspace => {
                    app.submission_input.pop();
                    app.submission_state = next_state;
                }
                KeyCode::Enter if !app.submission_input.trim().is_empty() => {
                    app.submission_state = SubmissionState::Confirming;
                }
                KeyCode::Enter => {
                    app.submission_state = next_state;
                }
                KeyCode::Esc => {
                    app.submission_state = SubmissionState::TypePicker;
                }
                _ => {
                    app.submission_state = next_state;
                }
            }
        }

        // ── TextPreview (content from $EDITOR) ───────────────────────
        SubmissionState::TextPreview => match code {
            KeyCode::Char('y') | KeyCode::Enter => {
                app.submission_state = SubmissionState::TextPreview;
                app.start_submission();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.submission_state = SubmissionState::TypePicker;
            }
            _ => {
                app.submission_state = SubmissionState::TextPreview;
            }
        },

        // ── Confirming (URL or file path) ─────────────────────────────
        SubmissionState::Confirming => match code {
            KeyCode::Char('y') | KeyCode::Enter => {
                app.submission_state = SubmissionState::Confirming;
                app.start_submission();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.submission_state = SubmissionState::TypePicker;
            }
            _ => {
                app.submission_state = SubmissionState::Confirming;
            }
        },

        // ── Done — any key dismisses ──────────────────────────────────
        SubmissionState::Done { .. } => {
            app.submission_state = SubmissionState::Hidden;
        }

        // ── Submitting — Esc cancels if the rx is still pending ───────
        SubmissionState::Submitting => {
            if code == KeyCode::Esc && app.submission_rx.is_some() {
                app.submission_rx = None;
                app.submission_state = SubmissionState::TypePicker;
            } else {
                app.submission_state = SubmissionState::Submitting;
            }
        }

        SubmissionState::Hidden => {}
    }
}
