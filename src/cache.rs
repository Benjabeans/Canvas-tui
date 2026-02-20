use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::models::{Assignment, CalendarEvent, Course, DiscussionTopic, User};

// ─── Cached payload ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheData {
    pub cached_at: DateTime<Utc>,
    pub user: Option<User>,
    pub courses: Vec<Course>,
    /// Flat list of (course_name, assignments) preserved from the API fetch.
    pub assignments: Vec<(String, Vec<Assignment>)>,
    pub calendar_events: Vec<CalendarEvent>,
    pub announcements: Vec<DiscussionTopic>,
}

// ─── Path ────────────────────────────────────────────────────────────────────

fn cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("canvas-tui").join("cache.json"))
}

// ─── I/O ─────────────────────────────────────────────────────────────────────

pub fn load_cache() -> Option<CacheData> {
    let path = cache_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn save_cache(data: &CacheData) -> Result<()> {
    let path = cache_path().ok_or_else(|| anyhow!("Could not determine cache directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(&path, json)?;
    Ok(())
}
