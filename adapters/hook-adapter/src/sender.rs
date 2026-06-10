//! HTTP POST with retry logic for AgentPulse events.

pub const MAX_RETRIES: u32 = 3;
pub const RETRY_DELAY_MS: u64 = 1000;

/// POST event JSON to AgentPulse server. Returns HTTP status code, or -1 on
/// complete failure after all retries.
pub fn send_event(data: &serde_json::Value) -> i32 {
    let url = std::env::var("AGENTPULSE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:17878/api/events".to_string());

    let timeout_secs: u64 = std::env::var("AGENTPULSE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let body = serde_json::to_vec(data).unwrap_or_default();

    for attempt in 1..=MAX_RETRIES {
        match ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .send_bytes(&body)
        {
            Ok(resp) => {
                let status = resp.status();
                if status == 201 {
                    log::info!("Event sent successfully (attempt {})", attempt);
                } else {
                    log::warn!("Server returned {} (attempt {})", status, attempt);
                }
                return status as i32;
            }
            Err(ureq::Error::Status(code, _resp)) => {
                log::warn!("Server returned {} (attempt {})", code, attempt);
                return code as i32;
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    log::warn!(
                        "Connection failed (attempt {}/{}): {}",
                        attempt, MAX_RETRIES, e
                    );
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                } else {
                    log::error!(
                        "Failed to send event after {} attempts: {}",
                        MAX_RETRIES, e
                    );
                    return -1;
                }
            }
        }
    }

    -1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_event_connection_refused_returns_minus_one() {
        // Use a URL that definitely won't respond on a random port.
        std::env::set_var("AGENTPULSE_URL", "http://127.0.0.1:19999/events");
        std::env::set_var("AGENTPULSE_TIMEOUT", "1");
        let data = serde_json::json!({"test": true});
        let status = send_event(&data);
        assert_eq!(status, -1);
    }
}
