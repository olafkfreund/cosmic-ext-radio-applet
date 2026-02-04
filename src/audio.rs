use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, warn};
use url::Url;

pub struct AudioManager {
    process: Arc<Mutex<Option<Child>>>,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
        }
    }

    /// Validates that a URL is safe to pass to mpv (http/https only)
    fn validate_url(url: &str) -> Result<(), &'static str> {
        match Url::parse(url) {
            Ok(parsed) => {
                let scheme = parsed.scheme();
                if scheme == "http" || scheme == "https" {
                    // Block localhost and private IP ranges
                    if let Some(host) = parsed.host_str() {
                        if host == "localhost"
                            || host == "127.0.0.1"
                            || host.starts_with("192.168.")
                            || host.starts_with("10.")
                            || host.starts_with("172.16.")
                        {
                            return Err("Local/private URLs not allowed");
                        }
                    }
                    Ok(())
                } else {
                    Err("Only http/https URLs are allowed")
                }
            }
            Err(_) => Err("Invalid URL format"),
        }
    }

    pub fn play(&self, url: String, volume: u8) {
        // Validate URL before passing to mpv (security)
        if let Err(e) = Self::validate_url(&url) {
            error!("Invalid stream URL: {} - {}", url, e);
            return;
        }

        self.stop(); // Stop current if any

        let child = Command::new("mpv")
            .arg("--no-video")
            .arg(format!("--volume={}", volume))
            .arg("--volume-max=200")
            .arg("--af=lavfi=[dynaudnorm]")
            .arg(&url)
            .spawn();

        debug!("Spawned mpv for {}", url);

        match child {
            Ok(child) => {
                if let Ok(mut guard) = self.process.lock() {
                    *guard = Some(child);
                }
            }
            Err(e) => {
                error!("Failed to start mpv: {}", e);
            }
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.process.lock() {
            if let Some(mut child) = guard.take() {
                if let Err(e) = child.kill() {
                    warn!("Failed to kill mpv process: {}", e);
                }
                let _ = child.wait();
            }
        }
    }

    pub fn set_volume(&self, _vol: f32) {
        // TODO: Implement volume control via mpv IPC socket
        // For now, volume is only set at stream start
    }
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop();
    }
}
