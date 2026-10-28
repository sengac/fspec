#[path = "automatic_compaction.rs"]
pub mod automatic;

use super::support::{chunks, TestResult};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use wiremock::{Mock, ResponseTemplate};

struct Environment {
    _directory: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

pub async fn actual_loop(fail: bool, abandon: bool) -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let _environment = Environment {
        previous: [
            "FSPEC_USER_DIR",
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
        ]
        .into_iter()
        .map(|name| (name, std::env::var_os(name)))
        .collect(),
        _directory: directory,
    };
    std::env::set_var("FSPEC_USER_DIR", _environment._directory.path());
    let mut fixture = automatic::Harness::new(false, false, false).await;
    let marker = fixture.directory.path().join("started");
    let command = format!("printf 'started' > '{}'; sleep 5", marker.display());
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let first = observed.fetch_add(1, Ordering::AcqRel) == 0;
            let delta = if first {
                json!({"role":"assistant","tool_calls":[{"index":0,"id":"running","type":"function",
                    "function":{"name":"Bash","arguments":json!({"command":command,"work_unit_id":"NONE"}).to_string()}}]})
            } else { json!({"role":"assistant","content":"new task response"}) };
            let frame = json!({"id":"settlement","object":"chat.completion.chunk","created":1,"model":"o3",
                "choices":[{"index":0,"delta":delta,"finish_reason":if first {"tool_calls"} else {"stop"}}],
                "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}});
            ResponseTemplate::new(200).insert_header("content-type","text/event-stream")
                .set_body_string(format!("data: {frame}\n\ndata: [DONE]\n\n"))
        }).with_priority(1).mount(&fixture.server).await;
    fixture.send();
    tokio::time::timeout(Duration::from_secs(10), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    let archive = fixture.directory.path().join("messages/messages.jsonl");
    let lease = if fail {
        std::fs::rename(
            &archive,
            fixture.directory.path().join("messages/saved.jsonl"),
        )?;
        std::fs::create_dir(&archive)?;
        None
    } else {
        let lease = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(fixture.directory.path().join("messages.lock"))?;
        lease.lock()?;
        Some(lease)
    };
    fixture.session.interrupt();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = fixture.chunks.recv().await?;
            let event = codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(&event);
            if event
                .to_string()
                .contains("Interruption requested; settling")
            {
                break;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    if fail {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if fixture.session.inner.try_lock().is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await?;
        let events = chunks(&fixture.session);
        let status = fixture.session.get_status();
        std::fs::remove_dir(&archive)?;
        std::fs::rename(
            fixture.directory.path().join("messages/saved.jsonl"),
            &archive,
        )?;
        codelet_agent_loop::persist::retry_pending_persistence(&fixture.session.id)?;
        fixture.task.abort();
        let _ = (&mut fixture.task).await;
        assert!(!events.iter().any(|event| event["type"] == "done"));
        assert_eq!(
            status,
            codelet_rpc_types::SessionStatus::Interrupted,
            "failed settlement incorrectly advertised Idle"
        );
        assert_eq!(requests.load(Ordering::Acquire), 1);
    } else if abandon {
        fixture.task.abort();
        let _ = (&mut fixture.task).await;
        let status = fixture.session.get_status();
        let events = chunks(&fixture.session);
        let wait = fixture.session.wait_for_settlement();
        tokio::pin!(wait);
        let admission_waited = std::future::poll_fn(|context| {
            use std::future::Future;
            std::task::Poll::Ready(wait.as_mut().poll(context).is_pending())
        })
        .await;
        drop(lease);
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let event = fixture.chunks.recv().await?;
                let event =
                    codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(&event);
                if event["type"] == "toolResult" {
                    break;
                }
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })
        .await??;
        wait.await;
        tokio::task::spawn_blocking({
            let id = fixture.session.id;
            move || codelet_agent_loop::persist::retry_pending_persistence(&id)
        })
        .await?
        .map_err(std::io::Error::other)?;
        assert_ne!(
            status,
            codelet_rpc_types::SessionStatus::Idle,
            "abandoned settlement advertised Idle before durable publication"
        );
        assert!(
            admission_waited,
            "replacement admission bypassed the settlement owner"
        );
        assert!(!events.iter().any(|event| event["type"] == "done"));
        assert_eq!(requests.load(Ordering::Acquire), 1);
    } else {
        fixture
            .session
            .send_input("genuinely new task".into(), None)?;
        tokio::task::yield_now().await;
        assert!(fixture.session.inner.try_lock().is_err());
        assert_eq!(requests.load(Ordering::Acquire), 1);
        assert_eq!(
            fixture.session.get_status(),
            codelet_rpc_types::SessionStatus::Running
        );
        drop(lease);
        fixture.until_done().await;
        fixture.until_done().await;
        assert_eq!(requests.load(Ordering::Acquire), 2);
        fixture.task.abort();
        let _ = (&mut fixture.task).await;
    }
    Ok(())
}
