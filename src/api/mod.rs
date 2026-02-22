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

    async fn post_json<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<Response, CanvasError> {
        let url = self.api_url(path).map_err(CanvasError::Other)?;
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await?;
        Self::check_status(resp).await
    }

    async fn check_status(resp: Response) -> Result<Response, CanvasError> {
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
                Err(CanvasError::RateLimited { retry_after: retry })
            }
            s if s.is_client_error() || s.is_server_error() => {
                let status = s.as_u16();
                let message = resp.text().await.unwrap_or_default();
                Err(CanvasError::Api { status, message })
            }
            _ => Ok(resp),
        }
    }

    async fn get_url(&self, url: Url) -> Result<Response, CanvasError> {
        let resp = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await?;
        Self::check_status(resp).await
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

    // ── Submission (create) ──────────────────────────────────────────────

    /// Submit an online text entry. The body is sent as HTML; plain text is
    /// wrapped in `<pre>` so whitespace and line breaks are preserved.
    pub async fn submit_text_entry(
        &self,
        course_id: u64,
        assignment_id: u64,
        text: &str,
    ) -> Result<Submission, CanvasError> {
        let html = format!(
            "<pre>{}</pre>",
            text.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        );
        let body = serde_json::json!({
            "submission": {
                "submission_type": "online_text_entry",
                "body": html
            }
        });
        let resp = self
            .post_json(
                &format!("/courses/{course_id}/assignments/{assignment_id}/submissions"),
                &body,
            )
            .await?;
        Ok(resp.json().await?)
    }

    /// Submit a URL.
    pub async fn submit_url(
        &self,
        course_id: u64,
        assignment_id: u64,
        url: &str,
    ) -> Result<Submission, CanvasError> {
        let body = serde_json::json!({
            "submission": {
                "submission_type": "online_url",
                "url": url
            }
        });
        let resp = self
            .post_json(
                &format!("/courses/{course_id}/assignments/{assignment_id}/submissions"),
                &body,
            )
            .await?;
        Ok(resp.json().await?)
    }

    /// Full three-step file upload + submission.
    pub async fn submit_file(
        &self,
        course_id: u64,
        assignment_id: u64,
        file_path: &std::path::Path,
    ) -> Result<Submission, CanvasError> {
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("upload");

        let data = std::fs::read(file_path).map_err(|e| {
            CanvasError::Other(anyhow::anyhow!("Cannot read '{}': {e}", file_path.display()))
        })?;
        let size = data.len() as u64;

        let content_type = mime_from_ext(file_path);

        // Step 1 — request an upload slot from Canvas.
        let slot_body = serde_json::json!({
            "name": filename,
            "size": size,
            "content_type": content_type
        });
        let slot_resp = self
            .post_json(
                &format!(
                    "/courses/{course_id}/assignments/{assignment_id}/submissions/self/files"
                ),
                &slot_body,
            )
            .await?;
        let slot: FileUploadSlot = slot_resp.json().await?;

        // Step 2 — upload bytes to the slot URL.
        // Use a client that does NOT follow redirects so we can re-add auth on
        // the Canvas confirmation redirect.
        let file_id = self.upload_bytes_to_slot(&slot, filename, content_type, data).await?;

        // Step 3 — create the submission with the uploaded file ID.
        let sub_body = serde_json::json!({
            "submission": {
                "submission_type": "online_upload",
                "file_ids": [file_id]
            }
        });
        let resp = self
            .post_json(
                &format!("/courses/{course_id}/assignments/{assignment_id}/submissions"),
                &sub_body,
            )
            .await?;
        Ok(resp.json().await?)
    }

    async fn upload_bytes_to_slot(
        &self,
        slot: &FileUploadSlot,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> Result<u64, CanvasError> {
        let param_name = slot.file_param.as_deref().unwrap_or("file").to_string();

        let mut form = reqwest::multipart::Form::new();
        for (k, v) in &slot.upload_params {
            form = form.text(k.clone(), v.clone());
        }
        let part = reqwest::multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str(content_type)
            .map_err(|e| CanvasError::Other(anyhow::anyhow!("Invalid content-type: {e}")))?;
        form = form.part(param_name, part);

        // Build a no-redirect client for the raw upload (S3/similar).
        let upload_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("canvas-tui/0.1.0")
            .build()
            .map_err(CanvasError::Network)?;

        let resp = upload_client
            .post(&slot.upload_url)
            .multipart(form)
            .send()
            .await?;

        let status = resp.status();

        if status.is_redirection() {
            // Canvas redirects us to a confirmation endpoint — follow it with auth.
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    CanvasError::Other(anyhow::anyhow!(
                        "Upload redirect missing Location header"
                    ))
                })?
                .to_string();

            let confirm = self
                .client
                .get(&location)
                .bearer_auth(&self.token)
                .send()
                .await?;
            let file: UploadedFile = confirm.json().await?;
            return Ok(file.id);
        }

        if status.is_success() {
            let file: UploadedFile = resp.json().await?;
            return Ok(file.id);
        }

        Err(CanvasError::Api {
            status: status.as_u16(),
            message: resp.text().await.unwrap_or_default(),
        })
    }
}

fn mime_from_ext(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "txt" | "md" | "rst" => "text/plain",
        "html" | "htm" => "text/html",
        "csv" => "text/csv",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "zip" => "application/zip",
        "py" => "text/x-python",
        "rs" => "text/x-rust",
        "js" => "text/javascript",
        "ts" => "text/typescript",
        "c" | "cpp" | "h" => "text/x-c",
        "java" => "text/x-java",
        _ => "application/octet-stream",
    }
}
