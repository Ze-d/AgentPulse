//! Cross-platform process tree detection.
//!
//! Walks up the parent chain from our PID to find the real agent process,
//! skipping shell wrappers (cmd.exe, powershell.exe, bash, etc.).

use sysinfo::{Pid, System};

#[cfg(target_os = "windows")]
const SHELL_NAMES: &[&str] = &[
    "cmd.exe", "powershell.exe", "pwsh.exe",
    "sh.exe", "bash.exe", "conhost.exe",
];

#[cfg(not(target_os = "windows"))]
const SHELL_NAMES: &[&str] = &["sh", "bash", "zsh", "fish", "dash"];

const AGENT_BINARIES: &[(&str, &str)] = &[
    ("node.exe", "claude-code"),
    ("codex.exe", "codex"),
    ("codex", "codex"),
    ("gemini", "gemini"),
    ("copilot", "copilot"),
];

/// Walk up the process tree and return `(pid, agent_source)`.
///
/// On Windows, skips shell wrappers to find the real agent.
/// On other platforms, returns the immediate parent PID.
pub fn detect() -> (u32, String) {
    let mut system = System::new_all();
    system.refresh_all();

    let my_pid = Pid::from(std::process::id() as usize);

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(me) = system.process(my_pid) {
            if let Some(parent) = me.parent() {
                return (parent.as_u32(), "claude-code".to_string());
            }
        }
        return (std::process::id(), "claude-code".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut current = my_pid;
        let mut last_non_shell = std::process::id();
        let detected_source = "claude-code".to_string();

        for _ in 0..5 {
            let process = match system.process(current) {
                Some(p) => p,
                None => break,
            };
            let parent_pid = match process.parent() {
                Some(p) => p,
                None => break,
            };
            let parent = match system.process(parent_pid) {
                Some(p) => p,
                None => break,
            };
            let name = parent.name().to_string_lossy().to_lowercase();
            if !SHELL_NAMES.contains(&name.as_str()) {
                last_non_shell = parent_pid.as_u32();
                for (bin, source) in AGENT_BINARIES {
                    if name == *bin {
                        return (parent_pid.as_u32(), source.to_string());
                    }
                }
            }
            current = parent_pid;
        }

        (last_non_shell, detected_source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_valid_pid() {
        let (pid, source) = detect();
        assert!(pid > 0, "pid should be positive, got {}", pid);
        assert!(!source.is_empty(), "source should not be empty");
    }

    #[test]
    fn test_detect_returns_known_source() {
        let (_pid, source) = detect();
        let valid_sources = ["claude-code", "codex", "gemini", "copilot", "unknown"];
        assert!(
            valid_sources.contains(&source.as_str()),
            "source '{}' should be one of {:?}",
            source,
            valid_sources
        );
    }
}
