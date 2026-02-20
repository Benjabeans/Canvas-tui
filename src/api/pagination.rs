use crate::models::PaginationLinks;
use reqwest::header::HeaderMap;

/// Parse the `Link` header returned by the Canvas API.
pub fn parse_link_header(headers: &HeaderMap) -> PaginationLinks {
    let mut links = PaginationLinks::default();

    let Some(header) = headers.get("link").and_then(|v| v.to_str().ok()) else {
        return links;
    };

    for part in header.split(',') {
        let mut segments = part.split(';');
        let url = segments
            .next()
            .map(|s| s.trim().trim_start_matches('<').trim_end_matches('>').to_string());
        let rel = segments.next().and_then(|s| {
            let s = s.trim();
            if s.starts_with("rel=") {
                Some(s.trim_start_matches("rel=").trim_matches('"').to_string())
            } else {
                None
            }
        });

        if let (Some(url), Some(rel)) = (url, rel) {
            match rel.as_str() {
                "current" => links.current = Some(url),
                "next" => links.next = Some(url),
                "prev" => links.prev = Some(url),
                "first" => links.first = Some(url),
                "last" => links.last = Some(url),
                _ => {}
            }
        }
    }

    links
}
