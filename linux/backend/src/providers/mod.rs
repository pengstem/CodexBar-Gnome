mod claude;
mod codex;

use crate::config::ResolvedConfig;
use crate::payload::ProviderSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Codex,
    Claude,
}

impl ProviderKind {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProviderFilter {
    All,
    One(ProviderKind),
}

impl ProviderFilter {
    fn matches(self, provider: ProviderKind) -> bool {
        match self {
            Self::All => true,
            Self::One(expected) => expected == provider,
        }
    }
}

pub fn parse_provider_filter(value: &str) -> Result<ProviderFilter, String> {
    match value {
        "all" => Ok(ProviderFilter::All),
        "codex" => Ok(ProviderFilter::One(ProviderKind::Codex)),
        "claude" => Ok(ProviderFilter::One(ProviderKind::Claude)),
        _ => Err(format!("unsupported provider filter: {value}")),
    }
}

pub fn collect_snapshots(config: &ResolvedConfig, filter: ProviderFilter) -> Vec<ProviderSnapshot> {
    let mut snapshots = Vec::new();

    for provider in [ProviderKind::Codex, ProviderKind::Claude] {
        if !filter.matches(provider) {
            continue;
        }

        if !config.provider_enabled(provider.slug()) {
            if matches!(filter, ProviderFilter::One(_)) {
                snapshots.push(disabled_snapshot(provider));
            }
            continue;
        }

        let snapshot = match provider {
            ProviderKind::Codex => codex::fetch(config),
            ProviderKind::Claude => claude::fetch(config),
        };
        snapshots.push(snapshot);
    }

    snapshots
}

fn disabled_snapshot(provider: ProviderKind) -> ProviderSnapshot {
    ProviderSnapshot {
        provider: provider.slug().to_string(),
        source: "disabled".to_string(),
        state: "error".to_string(),
        usage: None,
        credits: None,
        identity: None,
        status: None,
        error: Some("provider is disabled in config".to_string()),
    }
}
