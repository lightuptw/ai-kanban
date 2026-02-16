use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

pub struct OpencodeManager {
    url: String,
    port: u16,
    http_client: reqwest::Client,
}

#[derive(Debug)]
pub enum OpencodeStatus {
    AlreadyRunning,
    Started,
    Failed(String),
}

impl OpencodeManager {
    pub fn new(opencode_url: &str) -> Self {
        let port = opencode_url
            .rsplit_once(':')
            .and_then(|(_, p)| p.trim_end_matches('/').parse::<u16>().ok())
            .unwrap_or(4096);

        Self {
            url: opencode_url.to_string(),
            port,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn ensure_running(&self) -> OpencodeStatus {
        if self.is_healthy().await {
            tracing::info!(url = self.url.as_str(), "opencode server already running");
            return OpencodeStatus::AlreadyRunning;
        }

        tracing::info!("opencode not reachable, attempting auto-start...");

        let binary_path = match self.find_binary().await {
            Some(path) => {
                tracing::info!(path = path.as_str(), "Found opencode binary");
                path
            }
            None => {
                tracing::info!("opencode not installed, attempting installation...");
                match self.install().await {
                    Ok(path) => {
                        tracing::info!(path = path.as_str(), "opencode installed successfully");
                        path
                    }
                    Err(e) => {
                        return OpencodeStatus::Failed(format!("Failed to install opencode: {e}"));
                    }
                }
            }
        };

        match self.start_server(&binary_path).await {
            Ok(()) => OpencodeStatus::Started,
            Err(e) => OpencodeStatus::Failed(format!("Failed to start opencode serve: {e}")),
        }
    }

    async fn is_healthy(&self) -> bool {
        self.http_client
            .get(format!("{}/health", self.url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn find_binary(&self) -> Option<String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let candidates = [
            format!("{home}/.opencode/bin/opencode"),
            "/usr/local/bin/opencode".into(),
            "/usr/bin/opencode".into(),
        ];

        for path in &candidates {
            if tokio::fs::metadata(path).await.is_ok() {
                return Some(path.clone());
            }
        }

        let output = Command::new("which")
            .arg("opencode")
            .output()
            .await
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }

        None
    }

    async fn install(&self) -> Result<String, String> {
        tracing::info!("Installing opencode via https://opencode.ai/install ...");

        let curl_output = Command::new("bash")
            .arg("-c")
            .arg("curl -fsSL https://opencode.ai/install | bash")
            .env("CI", "true")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run install script: {e}"))?;

        if !curl_output.status.success() {
            let stderr = String::from_utf8_lossy(&curl_output.stderr);
            return Err(format!("opencode install failed: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&curl_output.stdout);
        tracing::debug!(output = stdout.as_ref(), "opencode install output");

        self.find_binary()
            .await
            .ok_or_else(|| "opencode binary not found after installation".into())
    }

    async fn start_server(&self, binary_path: &str) -> Result<(), String> {
        tracing::info!(
            port = self.port,
            binary = binary_path,
            "Starting opencode serve..."
        );

        let child = Command::new(binary_path)
            .arg("serve")
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--hostname")
            .arg("127.0.0.1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .kill_on_drop(false)
            .spawn()
            .map_err(|e| format!("Failed to spawn opencode serve: {e}"))?;

        let pid = child.id().unwrap_or(0);
        tracing::info!(pid = pid, port = self.port, "opencode serve process spawned");

        // Detach — we don't await the child; it runs as a background daemon
        std::mem::forget(child);

        // Poll for health with exponential backoff
        let mut delay_ms = 200;
        for attempt in 1..=15 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;

            if self.is_healthy().await {
                tracing::info!(
                    attempt = attempt,
                    url = self.url.as_str(),
                    "opencode serve is healthy"
                );
                return Ok(());
            }

            tracing::debug!(
                attempt = attempt,
                delay_ms = delay_ms,
                "opencode not ready yet, retrying..."
            );
            delay_ms = (delay_ms * 2).min(5000);
        }

        Err(format!(
            "opencode serve started (pid {pid}) but health check failed after 15 attempts"
        ))
    }
}
