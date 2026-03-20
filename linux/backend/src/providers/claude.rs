use std::fs;
use std::path::PathBuf;
use std::thread;

use serde::Deserialize;

use crate::config::ResolvedConfig;
use crate::payload::{
    CreditsSnapshot, IdentitySnapshot, ProviderSnapshot, RateWindowSnapshot, StatusSnapshot,
    UsageSnapshot,
};
use crate::status;
use crate::util;

#[derive(Debug, Deserialize)]
struct ClaudeCredentialRoot {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<ClaudeOAuthFileCredentials>,
}

#[derive(Debug, Deserialize)]
struct ClaudeOAuthFileCredentials {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "refreshToken")]
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    #[allow(dead_code)]
    expires_at_millis: Option<f64>,
    scopes: Option<Vec<String>>,
    #[serde(rename = "rateLimitTier")]
    rate_limit_tier: Option<String>,
}

#[derive(Debug)]
struct ClaudeOAuthCredentials {
    access_token: String,
    scopes: Vec<String>,
    rate_limit_tier: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsageResponse {
    #[serde(rename = "five_hour")]
    five_hour: Option<ClaudeWindow>,
    #[serde(rename = "seven_day")]
    seven_day: Option<ClaudeWindow>,
    #[serde(rename = "seven_day_sonnet")]
    seven_day_sonnet: Option<ClaudeWindow>,
    #[serde(rename = "seven_day_opus")]
    seven_day_opus: Option<ClaudeWindow>,
    #[serde(rename = "extra_usage")]
    extra_usage: Option<ClaudeExtraUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeWindow {
    utilization: Option<f64>,
    #[serde(rename = "resets_at")]
    resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeExtraUsage {
    #[serde(rename = "is_enabled")]
    is_enabled: Option<bool>,
    #[serde(rename = "monthly_limit")]
    monthly_limit: Option<f64>,
    #[serde(rename = "used_credits")]
    used_credits: Option<f64>,
}

pub fn fetch(config: &ResolvedConfig) -> ProviderSnapshot {
    let (usage_result, status_snapshot) = thread::scope(|s| {
        let status_handle = s.spawn(|| status::fetch_claude_status());
        let usage_result = fetch_inner(config);
        let status_snapshot = status_handle.join().unwrap_or_else(|_| fallback_status());
        (usage_result, status_snapshot)
    });

    match usage_result {
        Ok(mut snapshot) => {
            snapshot.status = Some(status_snapshot);
            snapshot
        }
        Err(error) => ProviderSnapshot {
            provider: "claude".to_string(),
            source: "oauth".to_string(),
            state: "error".to_string(),
            usage: None,
            credits: None,
            identity: None,
            status: Some(status_snapshot),
            error: Some(error),
        },
    }
}

fn fallback_status() -> StatusSnapshot {
    StatusSnapshot {
        indicator: "unknown".to_string(),
        description: "Status unavailable".to_string(),
        url: "https://status.claude.com/".to_string(),
    }
}

fn fetch_inner(config: &ResolvedConfig) -> Result<ProviderSnapshot, String> {
    if let Some(source) = config.provider_source("claude") {
        match source {
            "auto" | "oauth" => {}
            unsupported => {
                return Err(format!(
                    "claude source '{unsupported}' is not supported by the Linux backend v1; use 'oauth' or 'auto'"
                ));
            }
        }
    }

    let credentials = load_credentials()?;
    ensure_profile_scope(&credentials)?;

    let headers = vec![
        ("Authorization", format!("Bearer {}", credentials.access_token)),
        ("Accept", "application/json".to_string()),
        ("Content-Type", "application/json".to_string()),
        ("anthropic-beta", "oauth-2025-04-20".to_string()),
        ("User-Agent", "claude-code/2.1.0".to_string()),
    ];

    let response = util::http_get("https://api.anthropic.com/api/oauth/usage", &headers)?;
    let usage_response = match response.status {
        200..=299 => serde_json::from_str::<ClaudeUsageResponse>(&response.body)
            .map_err(|error| format!("failed to parse Claude OAuth response: {error}"))?,
        401 => {
            return Err("Claude OAuth token is unauthorized. Run `claude` to re-authenticate.".to_string());
        }
        status_code => {
            return Err(format!(
                "Claude OAuth usage API returned HTTP {status_code}: {}",
                util::truncate(&response.body, 280)
            ));
        }
    };

    let primary = usage_response
        .five_hour
        .as_ref()
        .and_then(|window| map_window(window, Some(5 * 60)))
        .ok_or_else(|| "Claude OAuth response did not contain a five_hour window.".to_string())?;
    let secondary = usage_response
        .seven_day
        .as_ref()
        .and_then(|window| map_window(window, Some(7 * 24 * 60)));
    let tertiary = usage_response
        .seven_day_sonnet
        .as_ref()
        .or(usage_response.seven_day_opus.as_ref())
        .and_then(|window| map_window(window, Some(7 * 24 * 60)));

    let usage = UsageSnapshot {
        primary,
        secondary,
        tertiary,
        updated_at_epoch_seconds: util::now_epoch_seconds(),
    };

    let credits = usage_response.extra_usage.as_ref().and_then(map_extra_usage);
    let identity = IdentitySnapshot {
        account_email: None,
        account_organization: None,
        login_method: map_login_method(credentials.rate_limit_tier.as_deref()),
    };

    Ok(ProviderSnapshot {
        provider: "claude".to_string(),
        source: "oauth".to_string(),
        state: "ok".to_string(),
        usage: Some(usage),
        credits,
        identity: Some(identity),
        status: None,
        error: None,
    })
}

fn load_credentials() -> Result<ClaudeOAuthCredentials, String> {
    let path = credentials_path()?;
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let parsed: ClaudeCredentialRoot = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let oauth = parsed
        .claude_ai_oauth
        .ok_or_else(|| format!("{} does not contain a claudeAiOauth block.", path.display()))?;

    let access_token = oauth
        .access_token
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} is missing accessToken.", path.display()))?;

    Ok(ClaudeOAuthCredentials {
        access_token,
        scopes: oauth.scopes.unwrap_or_default(),
        rate_limit_tier: oauth.rate_limit_tier,
    })
}

fn credentials_path() -> Result<PathBuf, String> {
    Ok(util::home_dir()?.join(".claude").join(".credentials.json"))
}

fn ensure_profile_scope(credentials: &ClaudeOAuthCredentials) -> Result<(), String> {
    let has_scope = credentials.scopes.iter().any(|scope| scope == "user:profile");

    if has_scope {
        Ok(())
    } else {
        Err("Claude OAuth token does not have the required 'user:profile' scope. Run `claude` to refresh credentials.".to_string())
    }
}

fn map_window(window: &ClaudeWindow, window_minutes: Option<u64>) -> Option<RateWindowSnapshot> {
    let used_percent = util::clamp_percent(window.utilization?);
    Some(RateWindowSnapshot {
        used_percent,
        remaining_percent: util::clamp_percent(100.0 - used_percent),
        window_minutes,
        reset_at_epoch_seconds: None,
        reset_at_iso8601: window.resets_at.clone(),
        reset_description: None,
    })
}

fn map_extra_usage(extra_usage: &ClaudeExtraUsage) -> Option<CreditsSnapshot> {
    if extra_usage.is_enabled != Some(true) {
        return None;
    }

    let monthly_limit = extra_usage.monthly_limit?;
    let used_credits = extra_usage.used_credits?;
    let remaining = (monthly_limit - used_credits) / 100.0;

    Some(CreditsSnapshot {
        remaining: remaining.max(0.0),
        updated_at_epoch_seconds: util::now_epoch_seconds(),
    })
}

fn map_login_method(rate_limit_tier: Option<&str>) -> Option<String> {
    let tier = rate_limit_tier?.trim().to_lowercase();
    if tier.contains("max") {
        return Some("Claude Max".to_string());
    }
    if tier.contains("pro") {
        return Some("Claude Pro".to_string());
    }
    if tier.contains("team") {
        return Some("Claude Team".to_string());
    }
    if tier.contains("enterprise") {
        return Some("Claude Enterprise".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_rate_limit_tiers_to_login_method() {
        assert_eq!(map_login_method(Some("default_claude_max_20x")).as_deref(), Some("Claude Max"));
        assert_eq!(map_login_method(Some("claude_pro")).as_deref(), Some("Claude Pro"));
        assert_eq!(map_login_method(Some("claude_team")).as_deref(), Some("Claude Team"));
        assert_eq!(
            map_login_method(Some("claude_enterprise")).as_deref(),
            Some("Claude Enterprise")
        );
    }
}
