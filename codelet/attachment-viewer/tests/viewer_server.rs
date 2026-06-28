#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/rust-attachment-viewer-server.feature
//
// Integration tests for the axum attachment viewer HTTP server. Each test maps
// 1:1 to a Gherkin scenario in the feature file. Tests use the fspec.pro harness
// pattern: build_router_with_config(ViewerConfig{cwd: tempdir}), bind
// 127.0.0.1:0, tokio::spawn(axum::serve(...)), and hit the server with reqwest.

use std::net::SocketAddr;
use std::path::PathBuf;

use codelet_attachment_viewer::{build_router_with_config, start_viewer, ViewerConfig};
use tempfile::TempDir;
use tokio::net::TcpListener;

/// Spawn the viewer router bound to `cwd` on a random local port; return the
/// base URL (e.g. http://127.0.0.1:PORT). The server runs for the test's life.
async fn spawn_router(cwd: PathBuf) -> String {
    let app = build_router_with_config(ViewerConfig { cwd });
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let addr: SocketAddr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://127.0.0.1:{}", addr.port())
}

#[tokio::test]
async fn render_a_markdown_attachment_as_an_html_page() {
    // @step Given a viewer server bound to a project directory containing a markdown file with a heading
    let tmp = TempDir::new().expect("tempdir");
    let dir = tmp.path().join("spec/attachments/RPC-001");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("design.md"), "# Hello Heading\n\nbody text\n").expect("write md");
    let base = spawn_router(tmp.path().to_path_buf()).await;

    // @step When I request that markdown file under the /view path
    let resp = reqwest::get(format!("{base}/view/spec/attachments/RPC-001/design.md"))
        .await
        .expect("request");

    // @step Then the response status is 200
    assert_eq!(resp.status().as_u16(), 200);

    // @step And the Content-Type is text/html
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(ct.starts_with("text/html"), "content-type was {ct}");

    let body = resp.text().await.expect("body");
    // @step And the body contains the rendered heading
    assert!(body.contains("<h1"), "no h1 in body");
    assert!(body.contains("Hello Heading"), "heading text missing");

    // @step And the document title is the file basename
    assert!(body.contains("<title>design.md</title>"), "title missing");
}

#[tokio::test]
async fn render_mermaid_code_blocks_for_client_side_rendering() {
    // @step Given a viewer server bound to a project directory containing a markdown file with a mermaid fenced code block
    let tmp = TempDir::new().expect("tempdir");
    std::fs::write(
        tmp.path().join("diagram.md"),
        "# Diagram\n\n```mermaid\ngraph TD\n  A-->B\n```\n",
    )
    .expect("write md");
    let base = spawn_router(tmp.path().to_path_buf()).await;

    // @step When I request that markdown file under the /view path
    let resp = reqwest::get(format!("{base}/view/diagram.md"))
        .await
        .expect("request");

    // @step Then the response status is 200
    assert_eq!(resp.status().as_u16(), 200);

    // @step And the body contains a pre element with class mermaid
    let body = resp.text().await.expect("body");
    assert!(
        body.contains("<pre class=\"mermaid\">"),
        "mermaid pre missing: {body}"
    );
}

#[tokio::test]
async fn serve_a_binary_image_attachment_raw_with_the_correct_content_type() {
    // @step Given a viewer server bound to a project directory containing a PNG image
    let tmp = TempDir::new().expect("tempdir");
    // Minimal valid-ish PNG signature + a few bytes.
    let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
    std::fs::write(tmp.path().join("logo.png"), &png_bytes).expect("write png");
    let base = spawn_router(tmp.path().to_path_buf()).await;

    // @step When I request that image under the /view path
    let resp = reqwest::get(format!("{base}/view/logo.png"))
        .await
        .expect("request");

    // @step Then the response status is 200
    assert_eq!(resp.status().as_u16(), 200);

    // @step And the Content-Type is image/png
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(ct, "image/png");

    // @step And the body is the raw image bytes
    let body = resp.bytes().await.expect("bytes");
    assert_eq!(body.as_ref(), png_bytes.as_slice());
}

#[tokio::test]
async fn block_directory_traversal_outside_the_project_directory() {
    // @step Given a viewer server bound to a project directory
    let tmp = TempDir::new().expect("tempdir");
    let base = spawn_router(tmp.path().to_path_buf()).await;
    let port: u16 = base
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .expect("port");

    // @step When I request a path that traverses above the project directory under /view
    // Send the literal `..` segments over a raw TCP request so the HTTP client
    // does not collapse them before they reach the server (the realistic attack).
    let status = raw_get_status(port, "/view/../../etc/passwd").await;

    // @step Then the response status is 403
    assert_eq!(status, 403);
}

/// Send a raw HTTP/1.1 GET with `path` verbatim (no client-side normalization)
/// and return the numeric status code from the response status line.
async fn raw_get_status(port: u16, path: &str) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.expect("write");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read");
    let text = String::from_utf8_lossy(&buf);
    let first = text.lines().next().unwrap_or_default();
    first
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0)
}

#[tokio::test]
async fn return_not_found_for_a_missing_file() {
    // @step Given a viewer server bound to a project directory
    let tmp = TempDir::new().expect("tempdir");
    let base = spawn_router(tmp.path().to_path_buf()).await;

    // @step When I request a markdown file that does not exist under /view
    let resp = reqwest::get(format!("{base}/view/does-not-exist.md"))
        .await
        .expect("request");

    // @step Then the response status is 404
    assert_eq!(resp.status().as_u16(), 404);
}

#[tokio::test]
async fn report_health() {
    // @step Given a viewer server bound to a project directory
    let tmp = TempDir::new().expect("tempdir");
    let base = spawn_router(tmp.path().to_path_buf()).await;

    // @step When I request the /health endpoint
    let resp = reqwest::get(format!("{base}/health"))
        .await
        .expect("request");

    // @step Then the response status is 200
    assert_eq!(resp.status().as_u16(), 200);

    // @step And the body indicates the server is ok
    let body = resp.text().await.expect("body");
    assert!(body.contains("ok"), "health body was {body}");
}

#[tokio::test]
async fn start_on_a_random_local_port_and_stop_cleanly() {
    // @step Given a project directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I start the viewer server for that directory
    let handle = start_viewer(tmp.path().to_path_buf())
        .await
        .expect("start viewer");

    // @step Then the returned handle exposes a non-zero port
    let port = handle.port;
    assert_ne!(port, 0, "port should be non-zero");

    // @step And a request to /health on that port succeeds
    let resp = reqwest::get(format!("http://127.0.0.1:{port}/health"))
        .await
        .expect("request");
    assert_eq!(resp.status().as_u16(), 200);

    // @step And after I stop the handle the server task ends
    handle.stop().await;
    // After shutdown, new connections must fail to reach the (now-closed) server.
    let after = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .timeout(std::time::Duration::from_millis(500))
        .send()
        .await;
    assert!(after.is_err(), "server should be unreachable after stop");
}
