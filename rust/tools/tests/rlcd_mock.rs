//! Shared test helper for the RLCD work units: a minimal in-process HTTP
//! server that emulates the RLCD (Jev/Simple-Jev v1) backend — `GET /health`
//! plus a configurable `POST /v1/classifier` response (status, extra
//! headers, body, delay, request-body capture) — on an ephemeral 127.0.0.1
//! port.
//!
//! Used by the RLCD-001 (service supervisor) and RLCD-002 (Decision tool)
//! test suites. Deliberately tiny: raw socket reads, one response per
//! connection, `Connection: close`, no keep-alive, no chunking — enough for
//! reqwest against `http://127.0.0.1:<port>`.
//!
//! Included as a module (`#[path = "rlcd_mock.rs"] mod rlcd_mock;`) from each
//! test binary so every feature keeps its 1:1 feature-file/test-file mapping
//! (exactly ONE linked test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Shared helper: not every method is used by every including suite.
#![allow(dead_code)]

use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

/// Default 422 fixture served when no /v1/classifier body is configured.
const DEFAULT_CLASSIFIER_422: &str = r#"{"error":{"message":"no fixture configured","type":"invalid_request_error","code":422}}"#;

/// Configurable /v1/classifier response (RLCD-002: status, headers, delay).
#[derive(Clone, Default)]
pub struct ClassifierResponse {
    /// Explicit HTTP status; `None` = infer (422 for error/default bodies,
    /// 200 otherwise — the RLCD-001 default semantics).
    pub status: Option<u16>,
    /// Response body (empty = default 422 fixture).
    pub body: String,
    /// Optional `Retry-After` header value (for 429).
    pub retry_after: Option<String>,
    /// Milliseconds to sleep before responding (for timeout tests).
    pub delay_ms: u64,
}

#[derive(Default)]
struct State {
    health: Option<String>,
    answer: ClassifierResponse,
    hits: usize,
    bodies: Vec<String>,
}

/// Mock RLCD backend. `health` is the `GET /health` response body; `None` =
/// accept the connection and close it (simulates a server that is not ready
/// yet). `answer` is the `/v1/classifier` response (status/headers/body/
/// delay); an empty body yields a 422 error fixture.
#[derive(Clone)]
pub struct MockRlcd {
    state: Arc<Mutex<State>>,
    stop_flag: Arc<AtomicBool>,
    stop_notify: Arc<Notify>,
    port: Arc<Mutex<Option<u16>>>,
}

impl MockRlcd {
    /// A mock reporting `{"status":"ready","model":"<model>"}` on /health.
    pub fn ready(model: &str) -> Self {
        Self::with_health(Some(&format!(
            r#"{{"status":"ready","model":"{model}"}}"#
        )))
    }

    /// A mock with an explicit /health body (or `None` = never ready).
    pub fn with_health(health_body: Option<&str>) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                health: health_body.map(str::to_string),
                ..Default::default()
            })),
            stop_flag: Arc::new(AtomicBool::new(false)),
            stop_notify: Arc::new(Notify::new()),
            port: Arc::new(Mutex::new(None)),
        }
    }

    /// Switch the /health response at runtime (ready -> not-ready, etc.).
    pub fn set_health(&self, body: Option<&str>) {
        let mut guard = self.state.lock().expect("mock lock poisoned");
        guard.health = body.map(str::to_string);
    }

    /// Set the body returned for /v1/classifier (status inferred: 422 for
    /// `{"error"...}` bodies, 200 otherwise).
    pub fn set_answers(&self, json: &str) {
        let mut guard = self.state.lock().expect("mock lock poisoned");
        guard.answer.body = json.to_string();
        guard.answer.status = None;
    }

    /// Set an explicit status + body for /v1/classifier (422/429/500 ...).
    pub fn set_status(&self, status: u16, body: &str) {
        let mut guard = self.state.lock().expect("mock lock poisoned");
        guard.answer.status = Some(status);
        guard.answer.body = body.to_string();
    }

    /// Set the `Retry-After` header sent with the /v1/classifier response.
    pub fn set_retry_after(&self, value: &str) {
        let mut guard = self.state.lock().expect("mock lock poisoned");
        guard.answer.retry_after = Some(value.to_string());
    }

    /// Delay the /v1/classifier response by N milliseconds (timeout tests).
    pub fn set_delay_ms(&self, ms: u64) {
        let mut guard = self.state.lock().expect("mock lock poisoned");
        guard.answer.delay_ms = ms;
    }

    /// How many /v1/classifier requests have arrived so far.
    pub fn classifier_hits(&self) -> usize {
        self.state.lock().expect("mock lock poisoned").hits
    }

    /// The captured /v1/classifier request bodies, in arrival order.
    pub fn classifier_bodies(&self) -> Vec<String> {
        self.state.lock().expect("mock lock poisoned").bodies.clone()
    }

    /// Base URL once [`start`](Self::start) has been called.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port())
    }

    /// The port the mock is listening on (available after [`start`]).
    pub fn port(&self) -> u16 {
        self.port
            .lock()
            .expect("mock lock poisoned")
            .expect("mock not started")
    }

    /// Start the server on an ephemeral port; returns once it is listening.
    pub async fn start(self) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let port = listener.local_addr().expect("local addr").port();
        *self.port.lock().expect("mock lock poisoned") = Some(port);

        let state = self.state.clone();
        let stop_flag = self.stop_flag.clone();
        let stop_notify = self.stop_notify.clone();

        tokio::spawn(async move {
            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                tokio::select! {
                    _ = stop_notify.notified() => break,
                    accepted = listener.accept() => {
                        let Ok((socket, _)) = accepted else {
                            break;
                        };
                        let state_c = state.clone();
                        tokio::spawn(async move {
                            let _ = handle_connection(socket, state_c).await;
                        });
                    }
                }
            }
        });

        self
    }

    /// Start the server on a SPECIFIC 127.0.0.1 port (the reuse/discovery
    /// scenarios need a controlled port inside a scan window). Returns the
    /// bind error on collision instead of panicking, so callers can retry
    /// on a fresh base port.
    pub async fn start_on_port(self, port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).await?;
        *self.port.lock().expect("mock lock poisoned") = Some(port);

        let state = self.state.clone();
        let stop_flag = self.stop_flag.clone();
        let stop_notify = self.stop_notify.clone();

        tokio::spawn(async move {
            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                tokio::select! {
                    _ = stop_notify.notified() => break,
                    accepted = listener.accept() => {
                        let Ok((socket, _)) = accepted else {
                            break;
                        };
                        let state_c = state.clone();
                        tokio::spawn(async move {
                            let _ = handle_connection(socket, state_c).await;
                        });
                    }
                }
            }
        });

        Ok(self)
    }

    /// Stop the accept loop (in-flight connections finish on their own).
    /// After this call the port is closed — /health probes fail with
    /// connection refused, simulating a stopped server.
    pub async fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.stop_notify.notify_waiters();
    }
}

/// Read the request (headers + body, per Content-Length) then answer.
/// `GET /health` -> health body (or accept-and-close when None); any other
/// request -> the configured /v1/classifier response.
async fn handle_connection(mut socket: TcpStream, state: Arc<Mutex<State>>) {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];
    for _ in 0..64 {
        let n = match socket.read(&mut chunk).await {
            Ok(n) => n,
            Err(_) => return,
        };
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some((head_end, body_len)) = head_bounds(&buf) {
            if buf.len() >= head_end + body_len {
                break;
            }
        }
    }

    let Some((head_end, body_len)) = head_bounds(&buf) else {
        return;
    };
    let text = String::from_utf8_lossy(&buf).into_owned();
    let head = &text[..head_end];
    let first_line = head.lines().next().unwrap_or_default();

    if first_line.contains("GET /health") {
        let body = {
            let guard = state.lock().expect("mock lock poisoned");
            guard.health.clone()
        };
        let Some(body) = body else {
            return; // accept-and-close: simulate "not ready yet"
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        return;
    }

    // /v1/classifier (or /v1/systemone): count + capture, then answer.
    let answer = {
        let mut guard = state.lock().expect("mock lock poisoned");
        guard.hits += 1;
        let body_start = head_end;
        let body_end = (head_end + body_len).min(text.len());
        guard.bodies.push(text[body_start..body_end].to_string());
        guard.answer.clone()
    };
    // the guard is dropped here (before the sleep below) so the spawned
    // task future stays Send

    if answer.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(answer.delay_ms)).await;
    }

    let body = if answer.body.is_empty() {
        DEFAULT_CLASSIFIER_422.to_string()
    } else {
        answer.body.clone()
    };
    let status = match answer.status {
        Some(s) => s,
        None => {
            if body.starts_with("{\"error\"") {
                422
            } else {
                200
            }
        }
    };
    let reason = if status < 400 { "OK" } else { "Error" };
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\n"
    );
    if let Some(retry_after) = &answer.retry_after {
        response.push_str(&format!("Retry-After: {retry_after}\r\n"));
    }
    response.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    let _ = socket.write_all(response.as_bytes()).await;
}

/// Locate the end of the HTTP head and the declared Content-Length.
/// Returns `(offset_just_past_headers, body_length)` once the head is fully
/// present in `buf`, else `None`.
fn head_bounds(buf: &[u8]) -> Option<(usize, usize)> {
    const NEEDLE: &[u8] = b"\r\n\r\n";
    let idx = buf
        .windows(NEEDLE.len())
        .position(|w| w == NEEDLE)?
        + NEEDLE.len();
    let head = String::from_utf8_lossy(&buf[..idx - NEEDLE.len()]);
    let mut body_len = 0usize;
    for line in head.lines() {
        if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
            body_len = value.trim().parse().unwrap_or(0);
        }
    }
    Some((idx, body_len))
}

/// Bind a plain TCP listener on an ephemeral port (a "non-RLCD" occupant).
/// The listener handle must stay alive for as long as the port must be
/// occupied; dropping it frees the port. (RLCD-001 suite; unused by 002.)
#[allow(dead_code)]
pub async fn occupy_port() -> (u16, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind occupant port");
    let port = listener.local_addr().expect("local addr").port();
    (port, listener)
}

/// A port that is free (almost certainly): bind 127.0.0.1:0, read the port
/// the kernel hands out, drop the listener — that port is now free.
/// (Available for suites that need a guaranteed-free port.)
#[allow(dead_code)]
pub async fn probe_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind probe port");
    listener.local_addr().expect("local addr").port()
}
