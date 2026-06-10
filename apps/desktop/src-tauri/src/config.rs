//! Configuration subsystem for AgentPulse.
//!
//! Reads settings from `{app_data_dir}/config.json` on startup. When the file
//! does not exist, a default config is generated so users can see and edit the
//! available options.
//!
//! Environment variables serve as **secondary overrides** (useful for CI or
//! containers where editing a config file is inconvenient):
//!
//! | Env var                       | Overrides           |
//! |-------------------------------|---------------------|
//! | `AGENTPULSE_PORT`             | `port`              |
//! | `AGENTPULSE_CHECK_INTERVAL`   | `check_interval_secs` |
//! | `AGENTPULSE_POLL_INTERVAL`    | `poll_interval_ms`  |

use serde::{Deserialize, Serialize};
use std::path::Path;

// ---------------------------------------------------------------------------
// Default values
// ---------------------------------------------------------------------------

fn default_port() -> u16 {
    17878
}
fn default_check_interval_secs() -> u64 {
    5
}
fn default_poll_interval_ms() -> u64 {
    2000
}

// ---------------------------------------------------------------------------
// Config struct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPulseConfig {
    /// HTTP port for the event server (127.0.0.1:{port}).
    #[serde(default = "default_port")]
    pub port: u16,

    /// Process checker interval in seconds.
    #[serde(default = "default_check_interval_secs")]
    pub check_interval_secs: u64,

    /// Frontend polling interval in milliseconds.
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

impl Default for AgentPulseConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            check_interval_secs: default_check_interval_secs(),
            poll_interval_ms: default_poll_interval_ms(),
        }
    }
}

impl AgentPulseConfig {
    /// Load config from `{app_data_dir}/config.json`, applying environment
    /// variable overrides on top.
    ///
    /// When the file is missing a default config is written so the user can
    /// discover and edit the available options.
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("config.json");

        let mut config = match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str::<Self>(&content).unwrap_or_else(|e| {
                tracing::warn!(
                    error = %e,
                    path = %path.display(),
                    "failed to parse config.json, using defaults"
                );
                Self::default()
            }),
            Err(_) => {
                let defaults = Self::default();
                if let Err(e) = defaults.save(app_data_dir) {
                    tracing::debug!(
                        error = %e,
                        "failed to write default config.json (non-fatal)"
                    );
                }
                defaults
            }
        };

        config.apply_env_overrides();
        config
    }

    /// Write the current config to `{app_data_dir}/config.json`.
    pub fn save(&self, app_data_dir: &Path) -> Result<(), String> {
        let _ = std::fs::create_dir_all(app_data_dir);
        let path = app_data_dir.join("config.json");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        tracing::info!(path = %path.display(), "config saved");
        Ok(())
    }

    /// Apply environment variable overrides (optional — for CI / containers).
    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("AGENTPULSE_PORT") {
            if let Ok(p) = v.parse() {
                tracing::debug!(port = p, "AGENTPULSE_PORT override");
                self.port = p;
            }
        }
        if let Ok(v) = std::env::var("AGENTPULSE_CHECK_INTERVAL") {
            if let Ok(s) = v.parse() {
                tracing::debug!(secs = s, "AGENTPULSE_CHECK_INTERVAL override");
                self.check_interval_secs = s;
            }
        }
        if let Ok(v) = std::env::var("AGENTPULSE_POLL_INTERVAL") {
            if let Ok(ms) = v.parse() {
                tracing::debug!(ms = ms, "AGENTPULSE_POLL_INTERVAL override");
                self.poll_interval_ms = ms;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let config = AgentPulseConfig::default();
        assert_eq!(config.port, 17878);
        assert_eq!(config.check_interval_secs, 5);
        assert_eq!(config.poll_interval_ms, 2000); // (was python.is_none())
        assert_eq!(config.poll_interval_ms, 2000);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = AgentPulseConfig::default();
        config.save(dir.path()).unwrap();

        let loaded = AgentPulseConfig::load(dir.path());
        assert_eq!(loaded.port, config.port);
        assert_eq!(loaded.check_interval_secs, config.check_interval_secs);
        assert_eq!(loaded.poll_interval_ms, config.poll_interval_ms);
    }

    #[test]
    fn test_load_missing_file_writes_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        assert!(!config_path.exists());

        let config = AgentPulseConfig::load(dir.path());
        // Default values used.
        assert_eq!(config.port, 17878);
        // Default config should have been written.
        assert!(config_path.exists());
    }

    #[test]
    fn test_load_corrupt_file_falls_back_to_defaults() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, "not valid json {{{").unwrap();

        let config = AgentPulseConfig::load(dir.path());
        assert_eq!(config.port, 17878); // fallback to default
    }

    #[test]
    fn test_custom_values_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = AgentPulseConfig {
            port: 9999,
            check_interval_secs: 10,
            poll_interval_ms: 5000,
        };
        config.save(dir.path()).unwrap();

        let loaded = AgentPulseConfig::load(dir.path());
        assert_eq!(loaded.port, 9999);
        assert_eq!(loaded.check_interval_secs, 10);
        assert_eq!(loaded.poll_interval_ms, 5000);
    }
}
