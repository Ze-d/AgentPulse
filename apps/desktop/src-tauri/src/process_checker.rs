use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::db::Database;

/// Spawn a background thread that periodically checks whether the Claude Code
/// process associated with each session is still alive. Dead processes have
/// their sessions removed from the database, which causes the frontend to
/// drop the card on the next poll cycle.
pub fn start(db: Arc<Mutex<Database>>) {
    thread::spawn(move || {
        let mut system = System::new();
        loop {
            thread::sleep(Duration::from_secs(5));

            let sessions = {
                let d = match db.lock() {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                match d.list_sessions_with_pid() {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("process_checker: list sessions: {e}");
                        continue;
                    }
                }
            };

            if sessions.is_empty() {
                continue;
            }

            system.refresh_processes(ProcessesToUpdate::All);

            for session in &sessions {
                let Some(pid) = session.pid else { continue };

                let alive = system.process(Pid::from(pid as usize)).is_some();

                if !alive {
                    log::info!(
                        "process_checker: PID {} gone, removing session {}",
                        pid,
                        session.session_id
                    );
                    let d = match db.lock() {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    if let Err(e) = d.delete_session(&session.session_id) {
                        log::error!("process_checker: delete session: {e}");
                    }
                }
            }
        }
    });
}
