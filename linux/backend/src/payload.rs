use serde::Serialize;

use crate::config::ResolvedConfig;
use crate::util;

#[derive(Debug, Serialize)]
pub struct AppPayload {
    pub app: &'static str,
    pub config_path: String,
    pub updated_at_epoch_seconds: u64,
    pub snapshots: Vec<ProviderSnapshot>,
}

impl AppPayload {
    pub fn new(config: &ResolvedConfig, snapshots: Vec<ProviderSnapshot>) -> Self {
        Self {
            app: "CodexBar-Gnome",
            config_path: config.active_path.display().to_string(),
            updated_at_epoch_seconds: util::now_epoch_seconds(),
            snapshots,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderSnapshot {
    pub provider: String,
    pub source: String,
    pub state: String,
    pub usage: Option<UsageSnapshot>,
    pub credits: Option<CreditsSnapshot>,
    pub identity: Option<IdentitySnapshot>,
    pub status: Option<StatusSnapshot>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UsageSnapshot {
    pub primary: RateWindowSnapshot,
    pub secondary: Option<RateWindowSnapshot>,
    pub tertiary: Option<RateWindowSnapshot>,
    pub updated_at_epoch_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct RateWindowSnapshot {
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub window_minutes: Option<u64>,
    pub reset_at_epoch_seconds: Option<u64>,
    pub reset_at_iso8601: Option<String>,
    pub reset_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreditsSnapshot {
    pub remaining: f64,
    pub updated_at_epoch_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct IdentitySnapshot {
    pub account_email: Option<String>,
    pub account_organization: Option<String>,
    pub login_method: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusSnapshot {
    pub indicator: String,
    pub description: String,
    pub url: String,
}
