mod config;
mod payload;
mod providers;
mod status;
mod util;

use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use payload::AppPayload;
use providers::{collect_snapshots, parse_provider_filter, ProviderFilter};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("poll");

    match command {
        "poll" => {
            let options = PollOptions::parse(&args[1..])?;
            emit_payload(options.provider_filter, options.pretty)
        }
        "watch" => {
            let options = WatchOptions::parse(&args[1..])?;
            loop {
                emit_payload(options.provider_filter, options.pretty)?;
                thread::sleep(Duration::from_secs(options.interval_seconds));
            }
        }
        "config-path" => {
            let resolved = config::ResolvedConfig::load()?;
            println!("{}", resolved.active_path.display());
            Ok(())
        }
        "--version" | "-V" | "version" => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn emit_payload(provider_filter: ProviderFilter, pretty: bool) -> Result<(), String> {
    let resolved = config::ResolvedConfig::load()?;
    let snapshots = collect_snapshots(&resolved, provider_filter);
    let payload = AppPayload::new(&resolved, snapshots);

    let output = if pretty {
        serde_json::to_string_pretty(&payload).map_err(|error| format!("failed to encode JSON: {error}"))?
    } else {
        serde_json::to_string(&payload).map_err(|error| format!("failed to encode JSON: {error}"))?
    };

    println!("{output}");
    Ok(())
}

struct PollOptions {
    provider_filter: ProviderFilter,
    pretty: bool,
}

impl PollOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut provider_filter = ProviderFilter::All;
        let mut pretty = false;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--provider" => {
                    let next = args
                        .get(index + 1)
                        .ok_or_else(|| "--provider requires a value".to_string())?;
                    provider_filter = parse_provider_filter(next)?;
                    index += 1;
                }
                "--pretty" => pretty = true,
                "--help" | "-h" => return Err(usage()),
                unknown => return Err(format!("unknown argument: {unknown}\n\n{}", usage())),
            }
            index += 1;
        }

        Ok(Self {
            provider_filter,
            pretty,
        })
    }
}

struct WatchOptions {
    provider_filter: ProviderFilter,
    interval_seconds: u64,
    pretty: bool,
}

impl WatchOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut provider_filter = ProviderFilter::All;
        let mut interval_seconds = 120;
        let mut pretty = false;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--provider" => {
                    let next = args
                        .get(index + 1)
                        .ok_or_else(|| "--provider requires a value".to_string())?;
                    provider_filter = parse_provider_filter(next)?;
                    index += 1;
                }
                "--interval" => {
                    let next = args
                        .get(index + 1)
                        .ok_or_else(|| "--interval requires a value".to_string())?;
                    interval_seconds = next
                        .parse::<u64>()
                        .map_err(|_| "--interval must be an integer number of seconds".to_string())?;
                    if interval_seconds == 0 {
                        return Err("--interval must be greater than zero".to_string());
                    }
                    index += 1;
                }
                "--pretty" => pretty = true,
                "--help" | "-h" => return Err(usage()),
                unknown => return Err(format!("unknown argument: {unknown}\n\n{}", usage())),
            }
            index += 1;
        }

        Ok(Self {
            provider_filter,
            interval_seconds,
            pretty,
        })
    }
}

fn usage() -> String {
    r#"Usage:
  codexbar-gnome-backend poll [--provider codex|claude|all] [--pretty]
  codexbar-gnome-backend watch [--provider codex|claude|all] [--interval <seconds>] [--pretty]
  codexbar-gnome-backend config-path
  codexbar-gnome-backend --version
"#
    .to_string()
}
