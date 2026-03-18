use serde::Deserialize;

use crate::payload::StatusSnapshot;
use crate::util;

#[derive(Debug, Deserialize)]
struct StatusPageResponse {
    status: StatusPageStatus,
}

#[derive(Debug, Deserialize)]
struct StatusPageStatus {
    indicator: String,
    description: String,
}

pub fn fetch_codex_status() -> StatusSnapshot {
    fetch_status_page(
        "https://status.openai.com/api/v2/status.json",
        "https://status.openai.com/",
    )
}

pub fn fetch_claude_status() -> StatusSnapshot {
    fetch_status_page(
        "https://status.claude.com/api/v2/status.json",
        "https://status.claude.com/",
    )
}

fn fetch_status_page(api_url: &str, public_url: &str) -> StatusSnapshot {
    let response = match util::http_get(api_url, &[("Accept", "application/json".to_string())]) {
        Ok(response) => response,
        Err(error) => return unknown_status(public_url, error),
    };

    if !(200..=299).contains(&response.status) {
        return unknown_status(
            public_url,
            format!("status API returned HTTP {}", response.status),
        );
    }

    match serde_json::from_str::<StatusPageResponse>(&response.body) {
        Ok(parsed) => StatusSnapshot {
            indicator: parsed.status.indicator,
            description: parsed.status.description,
            url: public_url.to_string(),
        },
        Err(error) => unknown_status(public_url, error.to_string()),
    }
}

fn unknown_status(public_url: &str, detail: String) -> StatusSnapshot {
    StatusSnapshot {
        indicator: "unknown".to_string(),
        description: format!("Status unavailable: {}", util::truncate(&detail, 120)),
        url: public_url.to_string(),
    }
}
