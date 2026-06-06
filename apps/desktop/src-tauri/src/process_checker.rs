use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::db::Database;
use crate::AgentStatus;

/// Returns true if the session status indicates it is still actively running.
/// Terminal states (Completed, Failed) and Unknown sessions should not be
/// cleaned up by the process checker — they are handled by the retention
/// cleanup instead.
fn is_active_status(status: &AgentStatus) -> bool {
    !matches!(
        status,
        AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Unknown
    )
}

/// Spawn a background thread that periodically checks whether the Claude Code
/// process associated with each active session is still alive. Dead processes
/// have their sessions removed from the database, which causes the frontend to
/// drop the card on the next poll cycle.
///
/// Sessions in terminal states (Completed, Failed) are left untouched — they
/// will be cleaned up later by the retention-based cleanup.
pub fn start(db: Arc<Mutex<Database>>) {
    thread::spawn(move || {
        let mut system = System::new();
        loop {
            thread::sleep(Duration::from_secs(5));

            let sessions = {
                let d = match db.lock() {
                    Ok(d) => d,
                    Err(_) => {
                        tracing::warn!("process_checker: DB lock poisoned, skipping cycle");
                        continue;
                    }
                };
                match d.list_sessions_with_pid() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(error = %e, "process_checker: failed to list sessions");
                        continue;
                    }
                }
            };

            if sessions.is_empty() {
                tracing::trace!("process_checker: no sessions with PIDs to check");
                continue;
            }

            tracing::trace!(
                count = sessions.len(),
                "process_checker: checking {} sessions",
                sessions.len()
            );

            system.refresh_processes(ProcessesToUpdate::All);

            for session in &sessions {
                let Some(pid) = session.pid else { continue };

                // Skip terminal sessions — they stay visible for history
                if !is_active_status(&session.status) {
                    continue;
                }

                let alive = system.process(Pid::from(pid as usize)).is_some();

                if !alive {
                    tracing::info!(
                        pid = pid,
                        session_id = %session.session_id,
                        status = ?session.status,
                        "process_checker: PID gone, removing session"
                    );
                    let d = match db.lock() {
                        Ok(d) => d,
                        Err(_) => {
                            tracing::warn!("process_checker: DB lock poisoned during delete");
                            continue;
                        }
                    };
                    if let Err(e) = d.delete_session(&session.session_id) {
                        tracing::error!(
                            error = %e,
                            session_id = %session.session_id,
                            "process_checker: failed to delete session"
                        );
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_active_status_excludes_terminal() {
        assert!(is_active_status(&AgentStatus::Starting));
        assert!(is_active_status(&AgentStatus::Running));
        assert!(is_active_status(&AgentStatus::ToolRunning));
        assert!(is_active_status(&AgentStatus::WaitingInput));
        assert!(is_active_status(&AgentStatus::WaitingPermission));
        assert!(!is_active_status(&AgentStatus::Completed));
        assert!(!is_active_status(&AgentStatus::Failed));
        assert!(!is_active_status(&AgentStatus::Unknown));
    }
}
