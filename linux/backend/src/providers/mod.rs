mod claude;
mod codex;

use std::thread;

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
    let targets: Vec<ProviderKind> = [ProviderKind::Codex, ProviderKind::Claude]
        .into_iter()
        .filter(|p| filter.matches(*p))
        .collect();

    thread::scope(|s| {
        let handles: Vec<_> = targets
            .iter()
            .map(|&provider| {
                s.spawn(move || {
                    if !config.provider_enabled(provider.slug()) {
                        if matches!(filter, ProviderFilter::One(_)) {
                            return Some(disabled_snapshot(provider));
                        }
                        return None;
                    }

                    let snapshot = match provider {
                        ProviderKind::Codex => codex::fetch(config),
                        ProviderKind::Claude => claude::fetch(config),
                    };
                    Some(snapshot)
                })
            })
            .collect();

        handles
            .into_iter()
            .filter_map(|h| h.join().ok().flatten())
            .collect()
    })
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
