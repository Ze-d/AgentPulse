//! Hook event handler: reads JSON from stdin, enriches with agent info,
//! and POSTs to the AgentPulse server.

use std::io::Read;

/// Read hook JSON from stdin, detect agent info, and either print to stdout
/// (`test_mode == true`) or POST to the AgentPulse event server.  Exits the
/// process on critical failure.
pub fn run(test_mode: bool, url_override: Option<&str>) {
    // --- read stdin ---
    let mut raw = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut raw) {
        log::error!("Failed to read stdin: {}", e);
        std::process::exit(1);
    }

    let raw = raw.trim().to_string();
    if raw.is_empty() {
        log::info!("No stdin data, skipping");
        std::process::exit(0);
    }

    // --- parse JSON ---
    let mut data: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to parse stdin as JSON: {}", e);
            std::process::exit(1);
        }
    };

    // --- enrich with agent info ---
    let (pid, source) = crate::agent::detect();
    log::debug!("detected agent: source={} pid={}", source, pid);
    data["process_pid"] = serde_json::json!(pid);
    data["agent_source"] = serde_json::json!(source);

    // --- test mode: print to stdout ---
    if test_mode {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
        std::process::exit(0);
    }

    // --- production: POST to server ---
    if let Some(url) = url_override {
        std::env::set_var("AGENTPULSE_URL", url);
    }

    let status = crate::sender::send_event(&data);
    if status < 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    // Integration test for hook::run is done via manual smoke test:
    //   echo '{"session_id":"t1"}' | agentpulse-hook --test
    // This avoids spawning the test binary recursively which would
    // create a fork bomb.
}
