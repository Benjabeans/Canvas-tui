use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Courses ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: u64,
    pub name: Option<String>,
    pub course_code: Option<String>,
    pub workflow_state: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub enrollments: Option<Vec<Enrollment>>,
    pub total_students: Option<u64>,
    pub term: Option<Term>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    #[serde(rename = "type")]
    pub enrollment_type: Option<String>,
    pub role: Option<String>,
    pub computed_current_score: Option<f64>,
    pub computed_current_grade: Option<String>,
    pub computed_final_score: Option<f64>,
    pub computed_final_grade: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Term {
    pub id: u64,
    pub name: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

// ─── Assignments ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub lock_at: Option<DateTime<Utc>>,
    pub unlock_at: Option<DateTime<Utc>>,
    pub points_possible: Option<f64>,
    pub course_id: Option<u64>,
    pub submission_types: Option<Vec<String>>,
    pub has_submitted_submissions: Option<bool>,
    pub html_url: Option<String>,
    pub published: Option<bool>,
    pub submission: Option<Submission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: Option<u64>,
    pub assignment_id: Option<u64>,
    pub user_id: Option<u64>,
    pub score: Option<f64>,
    pub grade: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub graded_at: Option<DateTime<Utc>>,
    pub workflow_state: Option<String>,
    pub late: Option<bool>,
    pub missing: Option<bool>,
    pub attempt: Option<u64>,
}

// ─── Calendar Events ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: u64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub context_code: Option<String>,
    pub workflow_state: Option<String>,
    pub all_day: Option<bool>,
    pub location_name: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub html_url: Option<String>,
    pub assignment: Option<AssignmentEventDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentEventDetail {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub points_possible: Option<f64>,
}

// ─── Announcements / Discussion Topics ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionTopic {
    pub id: u64,
    pub title: Option<String>,
    pub message: Option<String>,
    pub posted_at: Option<DateTime<Utc>>,
    pub delayed_post_at: Option<DateTime<Utc>>,
    pub user_name: Option<String>,
    pub discussion_subentry_count: Option<u64>,
    pub read_state: Option<String>,
    pub unread_count: Option<u64>,
    pub html_url: Option<String>,
    pub is_announcement: Option<bool>,
    pub context_code: Option<String>,
}

// ─── User / Profile ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub login_id: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

// ─── Grades ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseGrade {
    pub course_id: u64,
    pub course_name: String,
    pub current_score: Option<f64>,
    pub current_grade: Option<String>,
    pub final_score: Option<f64>,
    pub final_grade: Option<String>,
}

// ─── File Upload ────────────────────────────────────────────────────────────

/// Returned by Canvas when you request an upload slot for a file submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadSlot {
    pub upload_url: String,
    #[serde(default)]
    pub upload_params: std::collections::HashMap<String, String>,
    /// The multipart field name Canvas expects the file bytes under.
    pub file_param: Option<String>,
}

/// Minimal file object returned by Canvas after a successful upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFile {
    pub id: u64,
    pub filename: Option<String>,
    pub size: Option<u64>,
    pub content_type: Option<String>,
}

// ─── Pagination ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct PaginationLinks {
    pub current: Option<String>,
    pub next: Option<String>,
    pub prev: Option<String>,
    pub first: Option<String>,
    pub last: Option<String>,
}
