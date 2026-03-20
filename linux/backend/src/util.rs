use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

pub fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

pub fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        let truncated = value.chars().take(max_chars).collect::<String>();
        format!("{truncated}…")
    }
}

pub fn decode_jwt_payload(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let decoded = decode_base64_url(payload)?;
    String::from_utf8(decoded).ok()
}

pub fn http_get(url: &str, headers: &[(&str, String)]) -> Result<HttpResponse, String> {
    let payload = PythonHttpRequest {
        url: url.to_string(),
        headers: headers
            .iter()
            .map(|(name, value)| PythonHeader {
                name: (*name).to_string(),
                value: value.clone(),
            })
            .collect(),
    };

    let request_body = serde_json::to_vec(&payload)
        .map_err(|error| format!("failed to encode HTTP request payload: {error}"))?;

    let mut command = Command::new("python3");
    command
        .arg("-c")
        .arg(PYTHON_HTTP_GET_SCRIPT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to launch python3 for HTTP request: {error}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&request_body)
            .map_err(|error| format!("failed to write HTTP request payload: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for python3 HTTP helper: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("python3 HTTP helper exited with {}", output.status)
        } else {
            format!("python3 HTTP helper failed: {stderr}")
        });
    }

    serde_json::from_slice::<PythonHttpResponse>(&output.stdout)
        .map(|response| HttpResponse {
            status: response.status,
            body: response.body,
        })
        .map_err(|error| format!("python3 HTTP helper returned invalid JSON: {error}"))
}

fn decode_base64_url(value: &str) -> Option<Vec<u8>> {
    let mut normalized = value.trim().to_string();
    let padding = normalized.len() % 4;
    if padding != 0 {
        normalized.push_str(&"=".repeat(4 - padding));
    }
    URL_SAFE_NO_PAD.decode(normalized.as_bytes()).ok()
}

#[derive(Serialize)]
struct PythonHttpRequest {
    url: String,
    headers: Vec<PythonHeader>,
}

#[derive(Serialize)]
struct PythonHeader {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct PythonHttpResponse {
    status: u16,
    body: String,
}

const PYTHON_HTTP_GET_SCRIPT: &str = r#"
import json
import ssl
import sys
import time
import urllib.error
import urllib.request

payload = json.load(sys.stdin)
context = ssl.create_default_context()
context.minimum_version = ssl.TLSVersion.TLSv1_2

last_error = None
for attempt in range(2):
    req = urllib.request.Request(payload['url'], method='GET')
    for header in payload.get('headers', []):
        req.add_header(header['name'], header['value'])
    req.add_header('Connection', 'close')

    try:
        with urllib.request.urlopen(req, timeout=8, context=context) as response:
            body = response.read().decode('utf-8', 'replace')
            json.dump({'status': response.status, 'body': body}, sys.stdout)
            sys.exit(0)
    except urllib.error.HTTPError as error:
        body = error.read().decode('utf-8', 'replace')
        json.dump({'status': error.code, 'body': body}, sys.stdout)
        sys.exit(0)
    except Exception as error:
        last_error = error
        time.sleep(0.3)

print(str(last_error), file=sys.stderr)
sys.exit(1)
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_strings_with_ellipsis() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello…");
    }
}
