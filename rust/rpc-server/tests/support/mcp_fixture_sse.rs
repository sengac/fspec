//! MCP-006: the fixture's legacy (2024-11-05 HTTP+SSE) MCP server.
//!
//! `GET /sse` opens an event stream whose first event is `endpoint`
//! (`/messages?session=N`); each `POST /messages?session=N` is answered 202
//! and its JSON-RPC response is pushed to that session's stream as a
//! `message` event. Modes: `Silent` never sends the endpoint event and
//! `Foreign` names an endpoint on another origin.

use super::{bind_and_announce, read_request, respond, write_http, FIXTURE_RESULT};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
pub enum Mode {
    Normal,
    Silent,
    Foreign,
}

type Streams = Arc<Mutex<HashMap<u64, TcpStream>>>;

pub fn serve(mode: Mode, bearer: Option<String>) -> std::io::Result<()> {
    let listener = bind_and_announce()?;
    let streams: Streams = Arc::new(Mutex::new(HashMap::new()));
    let next = Arc::new(AtomicU64::new(1));
    for stream in listener.incoming() {
        let stream = stream?;
        let (bearer, streams, next) = (bearer.clone(), Arc::clone(&streams), Arc::clone(&next));
        std::thread::spawn(move || {
            let _ = handle(stream, mode, bearer.as_deref(), &streams, &next);
        });
    }
    Ok(())
}

fn handle(
    stream: TcpStream,
    mode: Mode,
    bearer: Option<&str>,
    streams: &Streams,
    next: &AtomicU64,
) -> std::io::Result<()> {
    let request = read_request(&stream)?;
    let mut stream = stream;
    if bearer.is_some() && request.authorization.as_deref() != bearer {
        return write_http(&mut stream, "401 Unauthorized", None);
    }
    if request.line.starts_with("GET /sse") {
        return event_stream(stream, mode, streams, next);
    }
    let session = request
        .line
        .split_whitespace()
        .nth(1)
        .and_then(|path| path.strip_prefix("/messages?session="))
        .and_then(|id| id.parse::<u64>().ok());
    let Some(session) = session.filter(|_| request.line.starts_with("POST ")) else {
        return write_http(&mut stream, "404 Not Found", None);
    };
    let message: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
    if let Some(response) = respond(&message, FIXTURE_RESULT) {
        let target = streams
            .lock()
            .map_err(|_| std::io::Error::other("streams lock poisoned"))?
            .get(&session)
            .map(TcpStream::try_clone)
            .transpose()?;
        let Some(mut target) = target else {
            return write_http(&mut stream, "404 Not Found", None);
        };
        write!(target, "event: message\ndata: {response}\n\n")?;
        target.flush()?;
    }
    write_http(&mut stream, "202 Accepted", None)
}

/// Answer `GET /sse`: announce the endpoint (per mode) and hold the stream
/// open until the client goes away.
fn event_stream(
    mut stream: TcpStream,
    mode: Mode,
    streams: &Streams,
    next: &AtomicU64,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
    )?;
    stream.flush()?;
    let session = next.fetch_add(1, Ordering::SeqCst);
    match mode {
        Mode::Silent => {}
        Mode::Foreign => write!(
            stream,
            "event: endpoint\ndata: http://other.invalid/messages\n\n"
        )?,
        Mode::Normal => {
            write!(
                stream,
                "event: endpoint\ndata: /messages?session={session}\n\n"
            )?;
            streams
                .lock()
                .map_err(|_| std::io::Error::other("streams lock poisoned"))?
                .insert(session, stream.try_clone()?);
        }
    }
    stream.flush()?;
    let mut buf = [0u8; 64];
    while stream.read(&mut buf)? > 0 {}
    if let Ok(mut map) = streams.lock() {
        map.remove(&session);
    }
    Ok(())
}
