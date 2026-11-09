//! MCP-003: a minimal, real MCP server for integration tests.
//!
//! Built only with `--features mcp-fixture` (never in production builds).
//! It speaks MCP JSON-RPC 2.0 and exposes one tool, `search`.
//!
//! - no arguments: newline-delimited JSON-RPC over stdio;
//! - `--argv ARG..` (MCP-005): the same stdio server, whose `search` tool
//!   returns the arguments after `--argv` as a JSON array of strings;
//! - `--exit`: exits immediately, before any handshake (a broken server);
//! - `--http`: a Streamable HTTP server on 127.0.0.1; prints its port on the
//!   first stdout line, answers each POSTed request with `application/json`,
//!   accepts notifications with 202 and refuses the optional GET stream with 405;
//! - `--http --bearer TOKEN` (MCP-004): the same server, but every request
//!   without `Authorization: Bearer TOKEN` is refused with 401;
//! - `--sse [--bearer TOKEN]`, `--sse-silent`, `--sse-foreign` (MCP-006): a
//!   legacy HTTP+SSE MCP server (see `mcp_fixture_sse.rs`).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

#[path = "mcp_fixture_sse.rs"]
mod sse;

/// The `Authorization` value required by `--bearer TOKEN` at argument 2.
fn bearer_arg() -> std::io::Result<Option<String>> {
    match std::env::args().nth(2).as_deref() {
        Some("--bearer") => {
            let token = std::env::args().nth(3).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "--bearer needs a token")
            })?;
            Ok(Some(format!("Bearer {token}")))
        }
        _ => Ok(None),
    }
}

/// Bind 127.0.0.1 on a free port and print the port on the first stdout line.
fn bind_and_announce() -> std::io::Result<TcpListener> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut stdout = std::io::stdout();
    writeln!(stdout, "{}", listener.local_addr()?.port())?;
    stdout.flush()?;
    Ok(listener)
}

fn main() -> std::io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("--exit") => Ok(()),
        Some("--http") => serve_http(bearer_arg()?),
        Some("--sse") => sse::serve(sse::Mode::Normal, bearer_arg()?),
        Some("--sse-silent") => sse::serve(sse::Mode::Silent, None),
        Some("--sse-foreign") => sse::serve(sse::Mode::Foreign, None),
        Some("--argv") => {
            let received: Vec<String> = std::env::args().skip(2).collect();
            serve_stdio(&json!(received).to_string(), Listing::Answer)
        }
        // MCP-010: completes the handshake, then never answers tools/list.
        Some("--no-tools") => serve_stdio(FIXTURE_RESULT, Listing::Ignore),
        _ => serve_stdio(FIXTURE_RESULT, Listing::Answer),
    }
}

const FIXTURE_RESULT: &str = "fixture result";

/// The JSON-RPC response for one request, or `None` for a notification;
/// `tools/call` answers with `call_text`.
fn respond(message: &Value, call_text: &str) -> Option<Value> {
    let id = message.get("id")?.clone();
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => {
            let version = message
                .pointer("/params/protocolVersion")
                .cloned()
                .unwrap_or_else(|| json!("2025-03-26"));
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "fspec-mcp-fixture", "version": "1.0.0" }
            })
        }
        "tools/list" => json!({
            "tools": [{
                "name": "search",
                "description": "Search the fixture notes",
                "inputSchema": {
                    "type": "object",
                    "properties": { "query": { "type": "string" } }
                }
            }]
        }),
        "tools/call" => json!({
            "content": [{ "type": "text", "text": call_text }],
            "isError": false
        }),
        "ping" => json!({}),
        _ => {
            return Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {method}") }
            }))
        }
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

/// Whether the stdio fixture answers tools/list.
#[derive(PartialEq)]
enum Listing {
    Answer,
    Ignore,
}

fn serve_stdio(call_text: &str, listing: Listing) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let lists = message.get("method").and_then(Value::as_str) == Some("tools/list");
        if lists && listing == Listing::Ignore {
            continue;
        }
        if let Some(response) = respond(&message, call_text) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn serve_http(bearer: Option<String>) -> std::io::Result<()> {
    let listener = bind_and_announce()?;
    for stream in listener.incoming() {
        let stream = stream?;
        let bearer = bearer.clone();
        std::thread::spawn(move || {
            let _ = handle_http(stream, bearer.as_deref());
        });
    }
    Ok(())
}

/// One parsed request: its request line, `Authorization` value and body.
struct Request {
    line: String,
    authorization: Option<String>,
    body: Vec<u8>,
}

fn read_request(stream: &TcpStream) -> std::io::Result<Request> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut content_length = 0usize;
    let mut authorization: Option<String> = None;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
            if name.eq_ignore_ascii_case("authorization") {
                authorization = Some(value.trim().to_string());
            }
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    Ok(Request {
        line,
        authorization,
        body,
    })
}

fn handle_http(stream: TcpStream, bearer: Option<&str>) -> std::io::Result<()> {
    let Request {
        line: request_line,
        authorization,
        body,
    } = read_request(&stream)?;
    let mut stream = stream;
    if bearer.is_some() && authorization.as_deref() != bearer {
        return write_http(&mut stream, "401 Unauthorized", None);
    }
    if !request_line.starts_with("POST ") {
        return write_http(&mut stream, "405 Method Not Allowed", None);
    }
    let message: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    match respond(&message, FIXTURE_RESULT) {
        Some(response) => write_http(&mut stream, "200 OK", Some(&response.to_string())),
        None => write_http(&mut stream, "202 Accepted", None),
    }
}

fn write_http(stream: &mut TcpStream, status: &str, body: Option<&str>) -> std::io::Result<()> {
    let body = body.unwrap_or("");
    let content_type = if body.is_empty() {
        String::new()
    } else {
        "Content-Type: application/json\r\n".to_string()
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\n{content_type}Mcp-Session-Id: fixture-session\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
}
