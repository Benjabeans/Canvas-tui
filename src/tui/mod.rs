pub mod event;
pub mod ui;

use crate::api::CanvasClient;
use crate::cache::{save_cache, CacheData};
use crate::models::*;
use chrono::{DateTime, Utc};
use ratatui::widgets::ListState as RListState;
use std::collections::HashSet;
use tokio::sync::oneshot;

// ─── Submission ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmissionKind {
    TextEntry,
    Url,
    FileUpload,
}

impl SubmissionKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TextEntry => "Text Entry  (opens $EDITOR)",
            Self::Url => "URL Submission",
            Self::FileUpload => "File Upload",
        }
    }

    pub fn from_api_type(s: &str) -> Option<Self> {
        match s {
            "online_text_entry" => Some(Self::TextEntry),
            "online_url" => Some(Self::Url),
            "online_upload" => Some(Self::FileUpload),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum SubmissionState {
    #[default]
    Hidden,
    /// Choosing which submission type to use.
    TypePicker,
    /// User is typing a URL.
    UrlInput,
    /// User is typing a file path.
    FileInput,
    /// Showing text from $EDITOR before confirming.
    TextPreview,
    /// Final y/n confirm screen (URL or file).
    Confirming,
    /// Background task running.
    Submitting,
    /// API returned — show result.
    Done { success: bool, message: String },
}

impl SubmissionState {
    pub fn is_hidden(&self) -> bool {
        matches!(self, Self::Hidden)
    }
}

pub struct SubmitResult {
    pub success: bool,
    pub message: String,
}

// ─── Assignment Sort ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentSort {
    DueDateAsc,
    DueDateDesc,
    Course,
    Status,
}

impl AssignmentSort {
    pub fn next(self) -> Self {
        match self {
            Self::DueDateAsc => Self::DueDateDesc,
            Self::DueDateDesc => Self::Course,
            Self::Course => Self::Status,
            Self::Status => Self::DueDateAsc,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DueDateAsc => "Due ↑",
            Self::DueDateDesc => "Due ↓",
            Self::Course => "Course",
            Self::Status => "Status",
        }
    }
}

// ─── Unified Schedule View Mode ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnifiedViewMode {
    CalendarView,
    ListView,
}

impl UnifiedViewMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::CalendarView => Self::ListView,
            Self::ListView => Self::CalendarView,
        }
    }
}

// ─── Background Fetch Result ─────────────────────────────────────────────────

pub struct FetchResult {
    pub user: Option<User>,
    pub courses: Vec<Course>,
    pub assignments: Vec<(String, Vec<Assignment>)>,
    pub calendar_events: Vec<CalendarEvent>,
    pub announcements: Vec<DiscussionTopic>,
    pub fetched_at: DateTime<Utc>,
    /// Non-fatal error message to show in the status bar.
    pub error: Option<String>,
}

// ─── Calendar Item ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CalendarItem {
    pub start_at: Option<DateTime<Utc>>,
    pub title: String,
    pub item_type: &'static str, // "event" or "assignment"
    pub course_name: Option<String>,
    pub status: Option<String>,
    /// Canvas assignment ID, set when this item originates from an assignment.
    pub assignment_id: Option<u64>,
}

// ─── Navigation ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Courses,
    Assignments,
    Announcements,
}

impl Tab {
    pub const ALL: [Tab; 4] = [
        Tab::Dashboard,
        Tab::Courses,
        Tab::Assignments,
        Tab::Announcements,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Courses => "Courses",
            Tab::Assignments => "Schedule",
            Tab::Announcements => "Announcements",
        }
    }

    pub fn next(&self) -> Tab {
        let idx = Tab::ALL.iter().position(|t| t == self).unwrap_or(0);
        Tab::ALL[(idx + 1) % Tab::ALL.len()]
    }

    pub fn prev(&self) -> Tab {
        let idx = Tab::ALL.iter().position(|t| t == self).unwrap_or(0);
        if idx == 0 {
            Tab::ALL[Tab::ALL.len() - 1]
        } else {
            Tab::ALL[idx - 1]
        }
    }
}

// ─── App State ──────────────────────────────────────────────────────────────

pub struct App {
    pub client: CanvasClient,
    pub running: bool,
    pub active_tab: Tab,

    // Data
    pub user: Option<User>,
    pub courses: Vec<Course>,
    pub assignments: Vec<(String, Vec<Assignment>)>,
    pub calendar_events: Vec<CalendarEvent>,
    pub calendar_items: Vec<CalendarItem>,
    pub announcements: Vec<DiscussionTopic>,

    // UI state
    pub course_list_state: ListState,
    pub dashboard_list_state: ListState,
    pub assignment_list_state: ListState,
    pub assignment_sort: AssignmentSort,
    pub unified_view_mode: UnifiedViewMode,
    pub focal_assignment_id: Option<u64>,
    pub calendar_list_state: ListState,
    pub announcement_list_state: ListState,

    // Course filter for assignments tab
    pub course_filter: HashSet<String>,
    pub show_course_filter: bool,
    pub filter_list_state: ListState,

    // Status
    pub status_message: String,
    pub loading: bool,
    pub needs_refresh: bool,
    pub cached_at: Option<DateTime<Utc>>,

    // Background fetch channel
    pub fetch_rx: Option<oneshot::Receiver<FetchResult>>,

    // Submission modal
    pub submission_state: SubmissionState,
    pub submission_kind: Option<SubmissionKind>,
    pub submission_supported_kinds: Vec<SubmissionKind>,
    pub submission_cursor: usize,
    /// Shared text buffer: holds URL, file path, or editor content depending on state.
    pub submission_input: String,
    /// (course_id, assignment_id) of the assignment being submitted.
    pub submission_target: Option<(u64, u64)>,
    /// Set by event handler; consumed by the main loop to launch $EDITOR.
    pub launch_editor: bool,
    pub submission_rx: Option<oneshot::Receiver<SubmitResult>>,

    // Incremented each frame; used to drive the loading spinner.
    pub frame_count: u64,
}

/// Tracks logical selection plus a persistent ratatui scroll offset.
///
/// `selected` is the index among *selectable* items (header rows excluded).
/// `inner` carries the ratatui scroll offset; render functions sync
/// `inner.selected` to the correct absolute item index before calling
/// `render_stateful_widget`, so ratatui adjusts the offset only when the
/// cursor reaches a viewport edge.
pub struct ListState {
    pub inner: RListState,
    pub selected: usize,
    pub len: usize,
}

impl ListState {
    pub fn new() -> Self {
        let mut inner = RListState::default();
        inner.select(Some(0));
        Self { inner, selected: 0, len: 0 }
    }

    /// Move down — clamped at the last item (no wrap-around).
    pub fn select_next(&mut self) {
        if self.len > 0 && self.selected + 1 < self.len {
            self.selected += 1;
        }
    }

    /// Move up — clamped at the first item (no wrap-around).
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }
    }
}

impl App {
    pub fn new(client: CanvasClient) -> Self {
        Self {
            client,
            running: true,
            active_tab: Tab::Dashboard,
            user: None,
            courses: Vec::new(),
            assignments: Vec::new(),
            calendar_events: Vec::new(),
            calendar_items: Vec::new(),
            announcements: Vec::new(),
            course_list_state: ListState::new(),
            dashboard_list_state: ListState::new(),
            assignment_list_state: ListState::new(),
            assignment_sort: AssignmentSort::DueDateAsc,
            unified_view_mode: UnifiedViewMode::CalendarView,
            focal_assignment_id: None,
            calendar_list_state: ListState::new(),
            announcement_list_state: ListState::new(),
            course_filter: HashSet::new(),
            show_course_filter: false,
            filter_list_state: ListState::new(),
            status_message: "Loading...".into(),
            loading: true,
            needs_refresh: false,
            cached_at: None,
            fetch_rx: None,
            submission_state: SubmissionState::Hidden,
            submission_kind: None,
            submission_supported_kinds: Vec::new(),
            submission_cursor: 0,
            submission_input: String::new(),
            submission_target: None,
            launch_editor: false,
            submission_rx: None,
            frame_count: 0,
        }
    }

    /// Populate app state from a previously saved cache without making any
    /// network requests.  After this call the UI is immediately usable.
    pub fn load_from_cache(&mut self, cache: CacheData) {
        self.user = cache.user;
        self.course_list_state.set_len(cache.courses.len());
        self.courses = cache.courses;

        self.assignments = cache.assignments;
        self.recount_filtered_assignments();

        self.calendar_events = cache.calendar_events;

        self.announcement_list_state.set_len(cache.announcements.len());
        self.announcements = cache.announcements;

        self.rebuild_calendar_items();
        self.focal_assignment_id = self.compute_focal_assignment_id();

        let cal_idx = self.find_today_calendar_idx();
        self.calendar_list_state.selected = cal_idx;
        let asgn_idx = self.find_today_assignment_idx();
        self.assignment_list_state.selected = asgn_idx;

        self.cached_at = Some(cache.cached_at);
        self.loading = false;

        let synced = cache
            .cached_at
            .with_timezone(&chrono::Local)
            .format("%b %d %H:%M");
        let name = self
            .user
            .as_ref()
            .and_then(|u| u.name.clone())
            .unwrap_or_else(|| "Student".into());
        self.status_message = format!(
            "Hi, {}! Showing cached data from {synced} — press r to refresh.",
            name
        );
    }

    /// Spawn a background task that fetches all Canvas data without blocking
    /// the event loop.  Call `poll_fetch_result` each frame to collect the
    /// result once the task finishes.  No-ops if a fetch is already running.
    pub fn start_fetch(&mut self) {
        if self.fetch_rx.is_some() {
            return;
        }
        let client = self.client.clone();
        let (tx, rx) = oneshot::channel();
        self.fetch_rx = Some(rx);
        self.loading = true;
        self.status_message = "Syncing in background…".into();
        tokio::spawn(async move {
            let result = fetch_canvas_data(client).await;
            let _ = tx.send(result);
        });
    }

    /// Check the background fetch channel without blocking.  Returns `true`
    /// and applies the result to app state when data has arrived.
    pub fn poll_fetch_result(&mut self) -> bool {
        let result = match self.fetch_rx.as_mut() {
            None => return false,
            Some(rx) => match rx.try_recv() {
                Ok(r) => r,
                Err(oneshot::error::TryRecvError::Empty) => return false,
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.fetch_rx = None;
                    return false;
                }
            },
        };
        self.fetch_rx = None;
        self.apply_fetch_result(result);
        true
    }

    fn apply_fetch_result(&mut self, result: FetchResult) {
        self.user = result.user;
        self.course_list_state.set_len(result.courses.len());
        self.courses = result.courses;

        self.assignments = result.assignments;
        self.recount_filtered_assignments();

        self.calendar_events = result.calendar_events;
        self.announcement_list_state.set_len(result.announcements.len());
        self.announcements = result.announcements;

        self.rebuild_calendar_items();
        self.focal_assignment_id = self.compute_focal_assignment_id();

        let cal_idx = self.find_today_calendar_idx();
        self.calendar_list_state.selected = cal_idx;
        let asgn_idx = self.find_today_assignment_idx();
        self.assignment_list_state.selected = asgn_idx;

        self.cached_at = Some(result.fetched_at);
        self.loading = false;

        if let Some(err) = result.error {
            self.status_message = format!("Sync error: {err}");
        } else {
            let name = self
                .user
                .as_ref()
                .and_then(|u| u.name.clone())
                .unwrap_or_else(|| "Student".into());
            let synced = result
                .fetched_at
                .with_timezone(&chrono::Local)
                .format("%b %d %H:%M");
            self.status_message = format!(
                "Welcome, {}! {} courses loaded. Synced {synced}.",
                name,
                self.courses.len()
            );
        }
    }

    pub fn rebuild_calendar_items(&mut self) {
        let now = chrono::Utc::now();

        // Assignment IDs already covered by API calendar events (to avoid duplicates).
        let event_assignment_ids: HashSet<u64> = self
            .calendar_events
            .iter()
            .filter_map(|e| e.assignment.as_ref().and_then(|a| a.id))
            .collect();

        let mut items: Vec<CalendarItem> = self
            .calendar_events
            .iter()
            .map(|e| CalendarItem {
                start_at: e.start_at,
                title: e.title.clone().unwrap_or_else(|| "Untitled".into()),
                item_type: if e.event_type.as_deref() == Some("assignment") {
                    "assignment"
                } else {
                    "event"
                },
                course_name: None,
                status: None,
                assignment_id: e.assignment.as_ref().and_then(|a| a.id),
            })
            .collect();

        // Merge in assignment due dates not already present.
        for (course_name, assignments) in &self.assignments {
            for assignment in assignments {
                if assignment.due_at.is_none() {
                    continue;
                }
                if event_assignment_ids.contains(&assignment.id) {
                    continue;
                }

                let status = if let Some(ref sub) = assignment.submission {
                    match sub.workflow_state.as_deref() {
                        Some("graded") => Some(
                            sub.score
                                .map(|s| {
                                    format!(
                                        "{:.1}/{}",
                                        s,
                                        assignment.points_possible.unwrap_or(0.0)
                                    )
                                })
                                .unwrap_or_else(|| "Graded".into()),
                        ),
                        Some("submitted") => Some("Submitted".into()),
                        _ => {
                            if assignment.due_at.map_or(false, |d| d < now) {
                                if sub.missing.unwrap_or(false) {
                                    Some("Missing!".into())
                                } else {
                                    Some("Past due".into())
                                }
                            } else {
                                None
                            }
                        }
                    }
                } else if assignment.due_at.map_or(false, |d| d < now) {
                    Some("Past due".into())
                } else {
                    None
                };

                items.push(CalendarItem {
                    start_at: assignment.due_at,
                    title: assignment.name.clone().unwrap_or_else(|| "Unnamed".into()),
                    item_type: "assignment",
                    course_name: Some(course_name.clone()),
                    status,
                    assignment_id: Some(assignment.id),
                });
            }
        }

        items.sort_by(|a, b| a.start_at.cmp(&b.start_at));
        self.calendar_list_state.set_len(items.len());
        self.calendar_items = items;
    }

    /// Jump the active tab's list to the first item on or after today.
    pub fn jump_to_today_active(&mut self) {
        if self.active_tab != Tab::Assignments {
            return;
        }
        match self.unified_view_mode {
            UnifiedViewMode::CalendarView => {
                let idx = self.find_today_calendar_idx();
                self.calendar_list_state.selected = idx;
            }
            UnifiedViewMode::ListView => {
                let idx = match self.assignment_sort {
                    AssignmentSort::DueDateAsc => self.find_today_assignment_idx(),
                    _ => 0,
                };
                self.assignment_list_state.selected = idx;
            }
        }
    }

    /// Returns the Canvas ID of the first upcoming, incomplete assignment
    /// (due today or later, not yet submitted/graded), used to highlight the
    /// most actionable item across all sort modes.
    fn compute_focal_assignment_id(&self) -> Option<u64> {
        let today = chrono::Utc::now().date_naive();
        let mut flat: Vec<&Assignment> = self
            .assignments
            .iter()
            .flat_map(|(_, a)| a.iter())
            .collect();
        flat.sort_unstable_by(|a, b| match (a.due_at, b.due_at) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
            (Some(x), Some(y)) => x.cmp(&y),
        });
        flat.iter()
            .find(|a| {
                let is_current = a.due_at.map(|d| d.date_naive() >= today).unwrap_or(false);
                if !is_current {
                    return false;
                }
                !matches!(
                    a.submission
                        .as_ref()
                        .and_then(|s| s.workflow_state.as_deref()),
                    Some("graded") | Some("submitted")
                )
            })
            .map(|a| a.id)
    }

    pub fn find_today_calendar_idx(&self) -> usize {
        let today = chrono::Utc::now().date_naive();
        self.calendar_items
            .iter()
            .position(|item| {
                item.start_at
                    .map(|d| d.date_naive() >= today)
                    .unwrap_or(false)
            })
            // If everything is in the past, land on the last item.
            .unwrap_or_else(|| self.calendar_items.len().saturating_sub(1))
    }

    pub fn find_today_assignment_idx(&self) -> usize {
        let today = chrono::Utc::now().date_naive();
        let mut flat: Vec<&Assignment> = self
            .assignments
            .iter()
            .flat_map(|(_, a)| a.iter())
            .collect();
        flat.sort_unstable_by(|a, b| match (a.due_at, b.due_at) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
            (Some(x), Some(y)) => x.cmp(&y),
        });
        flat.iter()
            .position(|a| a.due_at.map(|d| d.date_naive() >= today).unwrap_or(false))
            .unwrap_or(0)
    }

    /// Returns the course name and assignment reference for the currently
    /// selected index, resolving correctly across all sort modes (flat and grouped).
    pub fn get_selected_assignment(&self) -> Option<(&str, &Assignment)> {
        let mut flat: Vec<(&str, &Assignment)> = self
            .assignments
            .iter()
            .filter(|(name, _)| self.course_passes_filter(name))
            .flat_map(|(course, assignments)| {
                assignments.iter().map(move |a| (course.as_str(), a))
            })
            .collect();

        match self.assignment_sort {
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
            AssignmentSort::Status => {
                flat.sort_by_key(|(_, a)| {
                    let now = chrono::Utc::now();
                    if let Some(ref sub) = a.submission {
                        match sub.workflow_state.as_deref() {
                            Some("graded") => 4u8,
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
                });
            }
            AssignmentSort::Course => { /* already in course order */ }
        }

        flat.into_iter().nth(self.assignment_list_state.selected)
    }

    /// Look up a full (course_name, &Assignment) by Canvas assignment ID.
    /// Used by the calendar view detail panel to show complete assignment data.
    pub fn get_assignment_by_id(&self, id: u64) -> Option<(&str, &Assignment)> {
        for (course_name, assignments) in &self.assignments {
            if let Some(a) = assignments.iter().find(|a| a.id == id) {
                return Some((course_name.as_str(), a));
            }
        }
        None
    }

    /// Returns the ordered list of course names that have assignments.
    pub fn assignment_course_names(&self) -> Vec<&str> {
        self.assignments.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// Returns true if the given course name passes the current filter.
    /// An empty filter set means "show all".
    pub fn course_passes_filter(&self, course_name: &str) -> bool {
        self.course_filter.is_empty() || self.course_filter.contains(course_name)
    }

    /// Toggle a course in the filter set.
    pub fn toggle_course_filter(&mut self, course_name: &str) {
        if self.course_filter.contains(course_name) {
            self.course_filter.remove(course_name);
        } else {
            self.course_filter.insert(course_name.to_string());
        }
        self.recount_filtered_assignments();
    }

    /// Recount visible assignments after filter change and clamp selection.
    pub fn recount_filtered_assignments(&mut self) {
        let total: usize = self
            .assignments
            .iter()
            .filter(|(name, _)| self.course_passes_filter(name))
            .map(|(_, a)| a.len())
            .sum();
        self.assignment_list_state.set_len(total);
    }

    /// Returns the course name and assignment for the currently selected
    /// dashboard upcoming item.
    pub fn get_selected_dashboard_assignment(&self) -> Option<(&str, &Assignment)> {
        let now = chrono::Utc::now();
        let one_month = now + chrono::Duration::days(30);
        let today = now.date_naive();

        let mut upcoming: Vec<(&str, &Assignment)> = self
            .assignments
            .iter()
            .flat_map(|(course, assignments)| {
                assignments.iter().map(move |a| (course.as_str(), a))
            })
            .filter(|(_, a)| {
                a.due_at.map(|d| d.date_naive() >= today && d <= one_month).unwrap_or(false)
            })
            .collect();

        upcoming.sort_by(|a, b| match (a.1.due_at, b.1.due_at) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
            (Some(x), Some(y)) => x.cmp(&y),
        });

        upcoming.into_iter().nth(self.dashboard_list_state.selected)
    }

    /// Open the submission modal for the currently selected assignment.
    /// Works in both CalendarView and ListView modes.
    /// Shows a status message and does nothing if the assignment doesn't
    /// support any online submission type we handle.
    pub fn open_submission_modal(&mut self) {
        // Extract (course_id, assignment_id, submission_types) without holding borrows.
        let resolved: Option<(u64, u64, Vec<String>)> =
            if self.unified_view_mode == UnifiedViewMode::CalendarView {
                // Calendar view: selected is an index into calendar_items.
                let cal_aid = self
                    .calendar_items
                    .get(self.calendar_list_state.selected)
                    .and_then(|item| item.assignment_id);

                match cal_aid {
                    None => {
                        self.status_message = "Selected item is not an assignment.".into();
                        return;
                    }
                    Some(aid) => {
                        // Collect what we need into owned values so the borrow ends here.
                        self.get_assignment_by_id(aid).map(|(_, a)| {
                            (
                                a.course_id.unwrap_or(0),
                                a.id,
                                a.submission_types.clone().unwrap_or_default(),
                            )
                        })
                    }
                }
            } else {
                // List view: use the existing helper.
                self.get_selected_assignment().map(|(_, a)| {
                    (
                        a.course_id.unwrap_or(0),
                        a.id,
                        a.submission_types.clone().unwrap_or_default(),
                    )
                })
            };

        let Some((course_id, assignment_id, types)) = resolved else {
            self.status_message = "No assignment selected.".into();
            return;
        };

        if course_id == 0 {
            self.status_message = "Cannot determine course for this assignment.".into();
            return;
        }

        let supported: Vec<SubmissionKind> = types
            .iter()
            .filter_map(|t| SubmissionKind::from_api_type(t.as_str()))
            .collect();

        if supported.is_empty() {
            self.status_message =
                "This assignment does not support online submission.".into();
            return;
        }

        self.submission_target = Some((course_id, assignment_id));
        self.submission_supported_kinds = supported;
        self.submission_cursor = 0;
        self.submission_input.clear();
        self.submission_kind = None;
        self.submission_state = SubmissionState::TypePicker;
    }

    /// Kick off a background submission task using the current
    /// `submission_kind` and `submission_input`.
    pub fn start_submission(&mut self) {
        let Some((course_id, assignment_id)) = self.submission_target else {
            return;
        };
        let Some(ref kind) = self.submission_kind.clone() else {
            return;
        };

        let content = self.submission_input.clone();
        let client = self.client.clone();
        let kind = kind.clone();

        let (tx, rx) = oneshot::channel();
        self.submission_rx = Some(rx);
        self.submission_state = SubmissionState::Submitting;

        tokio::spawn(async move {
            let result: Result<(), String> = match kind {
                SubmissionKind::TextEntry => client
                    .submit_text_entry(course_id, assignment_id, &content)
                    .await
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                SubmissionKind::Url => client
                    .submit_url(course_id, assignment_id, &content)
                    .await
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                SubmissionKind::FileUpload => {
                    let path = std::path::Path::new(&content);
                    client
                        .submit_file(course_id, assignment_id, path)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                }
            };

            let submit_result = match result {
                Ok(()) => SubmitResult {
                    success: true,
                    message: "Submission successful! Press any key to close.".into(),
                },
                Err(e) => SubmitResult {
                    success: false,
                    message: format!("Submission failed: {e}"),
                },
            };

            let _ = tx.send(submit_result);
        });
    }

    /// Poll for a completed background submission. Returns true when done.
    pub fn poll_submission_result(&mut self) -> bool {
        let result = match self.submission_rx.as_mut() {
            None => return false,
            Some(rx) => match rx.try_recv() {
                Ok(r) => r,
                Err(oneshot::error::TryRecvError::Empty) => return false,
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.submission_rx = None;
                    return false;
                }
            },
        };
        self.submission_rx = None;
        let success = result.success;
        self.submission_state = SubmissionState::Done {
            success,
            message: result.message,
        };
        if success {
            self.needs_refresh = true;
        }
        true
    }

    pub fn active_list_state_mut(&mut self) -> &mut ListState {
        match self.active_tab {
            Tab::Dashboard => &mut self.dashboard_list_state,
            Tab::Courses => &mut self.course_list_state,
            Tab::Assignments => match self.unified_view_mode {
                UnifiedViewMode::CalendarView => &mut self.calendar_list_state,
                UnifiedViewMode::ListView => &mut self.assignment_list_state,
            },
            Tab::Announcements => &mut self.announcement_list_state,
        }
    }
}

// ─── Background fetch (runs in a spawned task) ───────────────────────────────

async fn fetch_canvas_data(client: CanvasClient) -> FetchResult {
    let mut result = FetchResult {
        user: None,
        courses: Vec::new(),
        assignments: Vec::new(),
        calendar_events: Vec::new(),
        announcements: Vec::new(),
        fetched_at: Utc::now(),
        error: None,
    };

    match client.get_self().await {
        Ok(user) => result.user = Some(user),
        Err(e) => {
            result.error = Some(format!("fetching profile: {e}"));
            return result;
        }
    }

    match client.list_courses().await {
        Ok(courses) => result.courses = courses,
        Err(e) => {
            result.error = Some(format!("fetching courses: {e}"));
            return result;
        }
    }

    for course in &result.courses {
        let name = course.name.clone().unwrap_or_else(|| "Unnamed".into());
        if let Ok(assignments) = client.list_assignments(course.id, true).await {
            if !assignments.is_empty() {
                result.assignments.push((name, assignments));
            }
        }
    }

    let now = Utc::now();
    let start = now.format("%Y-%m-%d").to_string();
    let end = (now + chrono::Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let context_codes: Vec<String> = result
        .courses
        .iter()
        .map(|c| format!("course_{}", c.id))
        .collect();

    if let Ok(mut events) = client
        .list_calendar_events(&context_codes, &start, &end)
        .await
    {
        if let Ok(deadlines) = client
            .list_upcoming_events(&context_codes, &start, &end)
            .await
        {
            events.extend(deadlines);
        }
        events.sort_by(|a, b| a.start_at.cmp(&b.start_at));
        result.calendar_events = events;
    }

    if let Ok(announcements) = client.list_announcements(&context_codes).await {
        result.announcements = announcements;
    }

    result.fetched_at = Utc::now();

    // Save cache from within the background task so the main thread never blocks.
    let cache = CacheData {
        cached_at: result.fetched_at,
        user: result.user.clone(),
        courses: result.courses.clone(),
        assignments: result.assignments.clone(),
        calendar_events: result.calendar_events.clone(),
        announcements: result.announcements.clone(),
    };
    if let Err(e) = save_cache(&cache) {
        result.error = Some(format!("saving cache: {e}"));
    }

    result
}
