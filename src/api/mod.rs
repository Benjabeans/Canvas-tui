mod pagination;

use anyhow::{Context, Result};
use reqwest::{Client, Response, StatusCode};
use url::Url;

use crate::models::*;
use pagination::parse_link_header;

// ─── Error types ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CanvasError {
    #[error("HTTP {status}: {message}")]
    Api { status: u16, message: String },
    #[error("Rate limited – retry after {retry_after:.1}s")]
    RateLimited { retry_after: f64 },
    #[error("Unauthorized – check your API token")]
    Unauthorized,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

// ─── Client ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CanvasClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl CanvasClient {
    pub fn new(base_url: &str, token: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)
            .with_context(|| format!("Invalid Canvas URL: {base_url}"))?;

        let client = Client::builder()
            .user_agent("canvas-tui/0.1.0")
            .build()?;

        Ok(Self {
            client,
            base_url,
            token: token.to_string(),
        })
    }

    fn api_url(&self, path: &str) -> Result<Url> {
        let full = format!("/api/v1{}", path);
        self.base_url
            .join(&full)
            .with_context(|| format!("Bad API path: {path}"))
    }

    async fn get(&self, path: &str) -> Result<Response, CanvasError> {
        let url = self.api_url(path).map_err(CanvasError::Other)?;
        self.get_url(url).await
    }

    async fn get_url(&self, url: Url) -> Result<Response, CanvasError> {
        let resp = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await?;

        match resp.status() {
            StatusCode::UNAUTHORIZED => Err(CanvasError::Unauthorized),
            StatusCode::FORBIDDEN => Err(CanvasError::Api {
                status: 403,
                message: "Forbidden – insufficient permissions".into(),
            }),
            StatusCode::TOO_MANY_REQUESTS => {
                let retry = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(1.0);
                Err(CanvasError::RateLimited {
                    retry_after: retry,
                })
            }
            s if s.is_client_error() || s.is_server_error() => {
                let status = s.as_u16();
                let message = resp.text().await.unwrap_or_default();
                Err(CanvasError::Api { status, message })
            }
            _ => Ok(resp),
        }
    }

    async fn get_paginated<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<(Vec<T>, Option<String>), CanvasError> {
        let mut url = self.api_url(path).map_err(CanvasError::Other)?;
        for (k, v) in params {
            url.query_pairs_mut().append_pair(k, v);
        }
        let resp = self.get_url(url).await?;
        let next = parse_link_header(resp.headers()).next;
        let items: Vec<T> = resp.json().await?;
        Ok((items, next))
    }

    async fn get_all_pages<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<Vec<T>, CanvasError> {
        let mut all = Vec::new();
        let (items, mut next) = self.get_paginated(path, params).await?;
        all.extend(items);

        while let Some(next_url) = next.take() {
            let url = Url::parse(&next_url)
                .map_err(|e| CanvasError::Other(anyhow::anyhow!("Bad pagination URL: {e}")))?;
            let resp = self.get_url(url).await?;
            next = parse_link_header(resp.headers()).next;
            let items: Vec<T> = resp.json().await?;
            all.extend(items);
        }

        Ok(all)
    }

    // ── Courses ─────────────────────────────────────────────────────────

    pub async fn list_courses(&self) -> Result<Vec<Course>, CanvasError> {
        self.get_all_pages(
            "/courses",
            &[
                ("enrollment_state", "active"),
                ("include[]", "total_students"),
                ("include[]", "term"),
                ("include[]", "enrollments"),
                ("per_page", "50"),
            ],
        )
        .await
    }

    pub async fn get_course(&self, course_id: u64) -> Result<Course, CanvasError> {
        let resp = self.get(&format!("/courses/{course_id}")).await?;
        Ok(resp.json().await?)
    }

    // ── Grades ──────────────────────────────────────────────────────────

    pub fn extract_grades(&self, courses: &[Course]) -> Vec<CourseGrade> {
        courses
            .iter()
            .filter_map(|c| {
                let enrollment = c.enrollments.as_ref()?.iter().find(|e| {
                    e.enrollment_type.as_deref() == Some("student")
                })?;
                Some(CourseGrade {
                    course_id: c.id,
                    course_name: c.name.clone().unwrap_or_else(|| "Unnamed".into()),
                    current_score: enrollment.computed_current_score,
                    current_grade: enrollment.computed_current_grade.clone(),
                    final_score: enrollment.computed_final_score,
                    final_grade: enrollment.computed_final_grade.clone(),
                })
            })
            .collect()
    }

    // ── Assignments ─────────────────────────────────────────────────────

    pub async fn list_assignments(
        &self,
        course_id: u64,
        include_submission: bool,
    ) -> Result<Vec<Assignment>, CanvasError> {
        let mut params: Vec<(&str, &str)> = vec![
            ("per_page", "50"),
            ("order_by", "due_at"),
        ];
        if include_submission {
            params.push(("include[]", "submission"));
        }
        self.get_all_pages(&format!("/courses/{course_id}/assignments"), &params)
            .await
    }

    pub async fn get_assignment(
        &self,
        course_id: u64,
        assignment_id: u64,
    ) -> Result<Assignment, CanvasError> {
        let resp = self
            .get(&format!(
                "/courses/{course_id}/assignments/{assignment_id}"
            ))
            .await?;
        Ok(resp.json().await?)
    }

    // ── Submissions ─────────────────────────────────────────────────────

    pub async fn list_my_submissions(
        &self,
        course_id: u64,
    ) -> Result<Vec<Submission>, CanvasError> {
        self.get_all_pages(
            &format!("/courses/{course_id}/students/submissions"),
            &[("student_ids[]", "self"), ("per_page", "50")],
        )
        .await
    }

    // ── Calendar ────────────────────────────────────────────────────────

    pub async fn list_calendar_events(
        &self,
        context_codes: &[String],
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<CalendarEvent>, CanvasError> {
        let mut params: Vec<(&str, &str)> = vec![
            ("start_date", start_date),
            ("end_date", end_date),
            ("per_page", "50"),
            ("type", "event"),
        ];
        for code in context_codes {
            params.push(("context_codes[]", code));
        }
        self.get_all_pages("/calendar_events", &params).await
    }

    pub async fn list_upcoming_events(
        &self,
        context_codes: &[String],
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<CalendarEvent>, CanvasError> {
        let mut params: Vec<(&str, &str)> = vec![
            ("start_date", start_date),
            ("end_date", end_date),
            ("per_page", "50"),
            ("type", "assignment"),
        ];
        for code in context_codes {
            params.push(("context_codes[]", code));
        }
        self.get_all_pages("/calendar_events", &params).await
    }

    // ── Announcements ───────────────────────────────────────────────────

    pub async fn list_announcements(
        &self,
        context_codes: &[String],
    ) -> Result<Vec<DiscussionTopic>, CanvasError> {
        let mut params: Vec<(&str, &str)> = vec![
            ("per_page", "25"),
            ("latest_only", "false"),
        ];
        for code in context_codes {
            params.push(("context_codes[]", code));
        }
        self.get_all_pages("/announcements", &params).await
    }

    // ── Discussion Topics ───────────────────────────────────────────────

    pub async fn list_discussions(
        &self,
        course_id: u64,
    ) -> Result<Vec<DiscussionTopic>, CanvasError> {
        self.get_all_pages(
            &format!("/courses/{course_id}/discussion_topics"),
            &[("per_page", "25")],
        )
        .await
    }

    // ── User / Profile ──────────────────────────────────────────────────

    pub async fn get_self(&self) -> Result<User, CanvasError> {
        let resp = self.get("/users/self").await?;
        Ok(resp.json().await?)
    }
}
