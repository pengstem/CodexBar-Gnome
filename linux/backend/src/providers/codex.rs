use std::env;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use crate::config::ResolvedConfig;
use crate::payload::{
    CreditsSnapshot, IdentitySnapshot, ProviderSnapshot, RateWindowSnapshot, UsageSnapshot,
};
use crate::status;
use crate::util;

#[derive(Debug, Deserialize)]
struct CodexAuthFile {
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
    tokens: Option<CodexTokenSet>,
}

#[derive(Debug, Deserialize)]
struct CodexTokenSet {
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    account_id: Option<String>,
}

#[derive(Debug)]
struct CodexCredentials {
    access_token: String,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    id_token: Option<String>,
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexUsageResponse {
    user_id: Option<String>,
    account_id: Option<String>,
    email: Option<String>,
    plan_type: Option<String>,
    rate_limit: Option<CodexRateLimit>,
    credits: Option<CodexCredits>,
}

#[derive(Debug, Deserialize)]
struct CodexRateLimit {
    primary_window: Option<CodexWindow>,
    secondary_window: Option<CodexWindow>,
}

#[derive(Debug, Deserialize)]
struct CodexWindow {
    used_percent: f64,
    reset_at: u64,
    limit_window_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct CodexCredits {
    #[serde(deserialize_with = "deserialize_optional_number")]
    balance: Option<f64>,
}

fn deserialize_optional_number<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| serde::de::Error::custom("invalid numeric value"))
            .map(Some),
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| serde::de::Error::custom("invalid numeric string"))
            }
        }
        _ => Err(serde::de::Error::custom("expected number or numeric string")),
    }
}

pub fn fetch(config: &ResolvedConfig) -> ProviderSnapshot {
    match fetch_inner(config) {
        Ok(snapshot) => snapshot,
        Err(error) => ProviderSnapshot {
            provider: "codex".to_string(),
            source: "oauth".to_string(),
            state: "error".to_string(),
            usage: None,
            credits: None,
            identity: None,
            status: Some(status::fetch_codex_status()),
            error: Some(error),
        },
    }
}

fn fetch_inner(config: &ResolvedConfig) -> Result<ProviderSnapshot, String> {
    if let Some(source) = config.provider_source("codex") {
        match source {
            "auto" | "oauth" => {}
            unsupported => {
                return Err(format!(
                    "codex source '{unsupported}' is not supported by the Linux backend v1; use 'oauth' or 'auto'"
                ));
            }
        }
    }

    let credentials = load_credentials()?;
    let request_url = resolve_usage_url()?;

    let mut headers = vec![
        ("Authorization", format!("Bearer {}", credentials.access_token)),
        ("Accept", "application/json".to_string()),
        ("User-Agent", "CodexBar-Gnome".to_string()),
    ];
    if let Some(account_id) = credentials.account_id.clone().filter(|value| !value.is_empty()) {
        headers.push(("ChatGPT-Account-Id", account_id));
    }

    let response = util::http_get(&request_url, &headers)?;
    let usage_response = match response.status {
        200..=299 => serde_json::from_str::<CodexUsageResponse>(&response.body)
            .map_err(|error| format!("failed to parse codex usage response: {error}"))?,
        401 | 403 => {
            return Err("Codex OAuth token is unauthorized. Run `codex` to re-authenticate.".to_string());
        }
        status_code => {
            return Err(format!(
                "Codex usage API returned HTTP {status_code}: {}",
                util::truncate(&response.body, 280)
            ));
        }
    };

    let plan = usage_response
        .plan_type
        .clone()
        .or_else(|| extract_chatgpt_plan(credentials.id_token.as_deref()));

    let identity = IdentitySnapshot {
        account_email: usage_response
            .email
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| extract_email(credentials.id_token.as_deref())),
        account_organization: usage_response
            .account_id
            .clone()
            .or_else(|| usage_response.user_id.clone())
            .filter(|value| !value.is_empty()),
        login_method: plan,
    };

    let primary = usage_response
        .rate_limit
        .as_ref()
        .and_then(|rate_limit| rate_limit.primary_window.as_ref())
        .map(map_window)
        .ok_or_else(|| "Codex usage response did not contain a primary window.".to_string())?;
    let secondary = usage_response
        .rate_limit
        .as_ref()
        .and_then(|rate_limit| rate_limit.secondary_window.as_ref())
        .map(map_window);

    let usage = UsageSnapshot {
        primary,
        secondary,
        tertiary: None,
        updated_at_epoch_seconds: util::now_epoch_seconds(),
    };

    let credits = usage_response
        .credits
        .and_then(|credits| credits.balance)
        .map(|remaining| CreditsSnapshot {
            remaining,
            updated_at_epoch_seconds: util::now_epoch_seconds(),
        });

    Ok(ProviderSnapshot {
        provider: "codex".to_string(),
        source: "oauth".to_string(),
        state: "ok".to_string(),
        usage: Some(usage),
        credits,
        identity: Some(identity),
        status: Some(status::fetch_codex_status()),
        error: None,
    })
}

fn load_credentials() -> Result<CodexCredentials, String> {
    let auth_path = codex_home()?.join("auth.json");
    let contents = fs::read_to_string(&auth_path)
        .map_err(|error| format!("failed to read {}: {error}", auth_path.display()))?;
    let parsed: CodexAuthFile = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", auth_path.display()))?;

    if let Some(tokens) = parsed.tokens {
        if let Some(access_token) = tokens
            .access_token
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return Ok(CodexCredentials {
                access_token,
                refresh_token: tokens.refresh_token,
                id_token: tokens.id_token,
                account_id: tokens.account_id,
            });
        }
    }

    if let Some(access_token) = parsed
        .openai_api_key
        .as_ref()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
    {
        return Ok(CodexCredentials {
            access_token,
            refresh_token: None,
            id_token: None,
            account_id: None,
        });
    }

    Err(format!(
        "{} does not contain a usable Codex access token.",
        auth_path.display()
    ))
}

fn resolve_usage_url() -> Result<String, String> {
    let config_path = codex_home()?.join("config.toml");
    let base_url = fs::read_to_string(&config_path)
        .ok()
        .and_then(|contents| parse_chatgpt_base_url(&contents))
        .unwrap_or_else(|| "https://chatgpt.com/backend-api".to_string());

    let normalized = normalize_base_url(&base_url);
    let path = if normalized.contains("/backend-api") {
        "/wham/usage"
    } else {
        "/api/codex/usage"
    };
    Ok(format!("{normalized}{path}"))
}

fn codex_home() -> Result<PathBuf, String> {
    match env::var("CODEX_HOME") {
        Ok(value) if !value.trim().is_empty() => Ok(PathBuf::from(value)),
        _ => Ok(util::home_dir()?.join(".codex")),
    }
}

fn parse_chatgpt_base_url(contents: &str) -> Option<String> {
    for raw_line in contents.lines() {
        let trimmed = raw_line.split('#').next()?.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key != "chatgpt_base_url" {
            continue;
        }

        let normalized = value
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_string();
        if !normalized.is_empty() {
            return Some(normalized);
        }
    }
    None
}

fn normalize_base_url(base_url: &str) -> String {
    let mut normalized = base_url.trim().trim_end_matches('/').to_string();
    if normalized.is_empty() {
        normalized = "https://chatgpt.com/backend-api".to_string();
    }

    if (normalized.starts_with("https://chatgpt.com") || normalized.starts_with("https://chat.openai.com"))
        && !normalized.contains("/backend-api")
    {
        normalized.push_str("/backend-api");
    }

    normalized
}

fn map_window(window: &CodexWindow) -> RateWindowSnapshot {
    let used_percent = util::clamp_percent(window.used_percent);
    RateWindowSnapshot {
        used_percent,
        remaining_percent: util::clamp_percent(100.0 - used_percent),
        window_minutes: Some(window.limit_window_seconds / 60),
        reset_at_epoch_seconds: Some(window.reset_at),
        reset_at_iso8601: None,
        reset_description: None,
    }
}

fn extract_email(id_token: Option<&str>) -> Option<String> {
    let payload = decode_jwt_payload(id_token?)?;
    payload
        .get("email")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("https://api.openai.com/profile")
                .and_then(Value::as_object)
                .and_then(|profile| profile.get("email"))
                .and_then(Value::as_str)
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_chatgpt_plan(id_token: Option<&str>) -> Option<String> {
    let payload = decode_jwt_payload(id_token?)?;
    payload
        .get("https://api.openai.com/auth")
        .and_then(Value::as_object)
        .and_then(|auth| auth.get("chatgpt_plan_type"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("chatgpt_plan_type").and_then(Value::as_str))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn decode_jwt_payload(token: &str) -> Option<Value> {
    let payload = util::decode_jwt_payload(token)?;
    serde_json::from_str(&payload).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chatgpt_base_url() {
        let contents = r#"
        # comment
        chatgpt_base_url = "https://chat.openai.com"
        "#;

        assert_eq!(
            parse_chatgpt_base_url(contents).as_deref(),
            Some("https://chat.openai.com")
        );
    }

    #[test]
    fn normalizes_backend_api_url() {
        assert_eq!(
            normalize_base_url("https://chatgpt.com"),
            "https://chatgpt.com/backend-api"
        );
    }

    #[test]
    fn parses_string_credit_balance() {
        let payload = r#"{ "balance": "0" }"#;
        let parsed: CodexCredits = serde_json::from_str(payload).expect("parse credits");
        assert_eq!(parsed.balance, Some(0.0));
    }
}
