use codelet_test_helpers::setup_temp_directory;
use std::path::Path;
use std::process::Command;
use tracing_subscriber::util::SubscriberInitExt;

pub fn child(name: &str, payloads: bool, mode: &str) -> (String, tempfile::TempDir) {
    let home = setup_temp_directory().expect("isolated log home");
    if payloads {
        let directory = home.path().join(".fspec/logs/private");
        std::fs::create_dir_all(&directory).expect("seed rotated log directory");
        for day in 1..=6 {
            std::fs::write(directory.join(format!("client.log.2000-01-{day:02}")), "")
                .expect("seed old rotation");
        }
    }
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args(["--exact", name, "--nocapture"])
        .env("BUG239_CHILD", "1")
        .env("BUG239_MODE", mode)
        .env("HOME", home.path())
        .env("RUST_LOG", "trace")
        .env_remove("FSPEC_LOG_PAYLOADS");
    if payloads {
        command.env("FSPEC_LOG_PAYLOADS", "1");
    }
    if mode == "invalid" {
        command.env("FSPEC_LOG_PAYLOADS", "true");
    }
    let output = command.output().expect("child process runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        format!(
            "{}{}",
            read_logs(home.path()),
            String::from_utf8_lossy(&output.stderr)
        ),
        home,
    )
}

fn read_logs(path: &Path) -> String {
    let Ok(entries) = std::fs::read_dir(path) else {
        return String::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            if entry.path().is_dir() {
                read_logs(&entry.path())
            } else {
                std::fs::read_to_string(entry.path()).unwrap_or_default()
            }
        })
        .collect()
}

pub async fn run_child() -> bool {
    if std::env::var_os("BUG239_CHILD").is_none() {
        return false;
    }
    let mode = std::env::var("BUG239_MODE").expect("child mode");
    let workspace = setup_temp_directory().expect("isolated daemon workspace");
    std::fs::create_dir_all(workspace.path().join("spec")).expect("watch directory");
    let service = if !matches!(mode.as_str(), "client" | "invalid" | "legacy") {
        Some(super::common::build_service(workspace.path()).expect("real logging service"))
    } else {
        None
    };
    match service.as_ref() {
        Some(_) if mode == "common" => {
            codelet_common::logging::init_logging(true).expect("common tracing")
        }
        Some(service) if mode == "combined" => {
            super::common::init_tracing_combined(service).expect("combined tracing")
        }
        Some(service) => super::common::init_tracing_daemon(service).expect("daemon tracing"),
        None if mode == "legacy" => {
            let file = std::fs::File::create(
                Path::new(&std::env::var("HOME").expect("isolated home")).join("legacy.log"),
            )
            .expect("synthetic legacy control");
            tracing_subscriber::fmt()
                .with_writer(std::sync::Mutex::new(file))
                .with_max_level(tracing::Level::TRACE)
                .with_ansi(false)
                .finish()
                .init();
        }
        None => super::common::init_tracing_client().expect("client tracing"),
    }
    super::emit();
    let text = r#"{"service":"session","type":"input","session_id":"session-239","data":{"message":"PROMPT_PRIVATE"}}"#;
    codelet_tools::bridge_relay::handle_multiplexed_inbound(
        text,
        uuid::Uuid::nil(),
        std::sync::Arc::new(|_| {}),
        None,
        None,
        None,
    )
    .await
    .expect("real bridge input emitter");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    true
}
