//! Feature: spec/features/exec-stdin-detector-fault-reporting.feature
//!
//! BUG-376 tests, moved out of tests.rs so each test file maps one feature (COV-075).

use super::test_support::*;
use super::*;
use std::sync::Mutex;

/// Captures formatted tracing output for the BUG-376 log assertions.
#[derive(Clone, Default)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|error| std::io::Error::other(error.to_string()))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
    type Writer = CapturedLog;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl CapturedLog {
    fn text(&self) -> Result<String, String> {
        let bytes = self.0.lock().map_err(|error| error.to_string())?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[test]
fn the_refire_cooldown_keeps_working_after_its_lock_was_poisoned() -> Result<(), String> {
    let _guard = lock_tests()?;
    // @step Given a thread panicked while holding the exec-stdin cooldown lock
    let poisoner = std::thread::spawn(|| {
        let _held = LAST_FIRE_MS.write();
        std::panic::resume_unwind(Box::new("poison the exec-stdin cooldown lock"));
    });
    if poisoner.join().is_ok() {
        return Err("the poisoning thread did not panic".to_string());
    }
    // @step When the detector records a fire for exec session "exec-poisoned"
    record_fire("exec-poisoned");
    // @step Then the detector does not fire again for "exec-poisoned" before the 30 second cooldown elapses
    let elapsed = cooldown_elapsed("exec-poisoned");
    reset_cooldown("exec-poisoned");
    if elapsed {
        return Err("the cooldown reads as elapsed right after a recorded fire".to_string());
    }
    Ok(())
}

// Current-thread runtime: the detector task runs on the test thread, so the
// thread-local capturing subscriber sees its log lines.
#[tokio::test(flavor = "current_thread")]
async fn a_request_for_a_session_without_a_callback_is_reported() -> Result<(), String> {
    // BUG-447: this test drives tools without a session manager.
    crate::facade::declare_unrestricted_tool_access("tests drive tools without a session manager");
    use rig::tool::Tool;

    let log = CapturedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::ERROR)
        .finish();
    let _subscriber = tracing::subscriber::set_default(subscriber);
    // @step Given an agent session has no exec-stdin request callback
    let agent = Uuid::new_v4();
    set_exec_stdin_request_callback(agent, None);
    // @step When a command run through the Bash tool in that session stays quiet long enough to fire the detector
    // The detector fires once quiet >= 3s on its 2s tick (~4s); the command
    // outlives that window.
    crate::bash::BashTool::new(agent)
        .call(crate::bash::BashArgs {
            command: "sleep 7".to_string(),
            cwd: None,
        })
        .await
        .map_err(|error| error.to_string())?;
    // @step Then an error naming the agent session and the exec session is logged
    let text = log.text()?;
    let reported = text.lines().any(|line| {
        line.contains("ERROR")
            && line.contains(&agent.to_string())
            && line.contains("exec_session_id=")
    });
    if !reported {
        return Err(format!(
            "no error names agent {agent} and its exec session:\n{text}"
        ));
    }
    Ok(())
}
