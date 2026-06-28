//! `GET /view/{*path}` handler — port of attachment-server.ts `/view/*`.
//!
//! Percent-decodes the captured wildcard, runs the traversal guard BEFORE
//! touching the filesystem (so missing-and-traversing → 403), then serves
//! markdown rendered to HTML or other files raw with an extension→content-type
//! map. Missing file → 404, any other error → 500. Never panics; no `unwrap()`
//! in the request path.

use std::path::Path;

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::markdown::{render_markdown, viewer_template};
use crate::state::ViewerState;

use super::path::validate_path;

/// Build a simple text response with an explicit status.
fn text_response(status: StatusCode, body: &'static str) -> Response {
    (status, body).into_response()
}

/// Map a lowercased file extension to a content type for raw file serving.
fn content_type_for(extension: &str) -> &'static str {
    match extension {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// The lowercased extension of `path` (without the dot), or empty string.
fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default()
}

/// The file basename of `path`, or the raw string if none.
fn basename_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

/// Serve a `.md`/`.markdown` file rendered to an HTML viewer page.
fn serve_markdown(path: &Path) -> Response {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let html = viewer_template(&basename_of(path), &render_markdown(&content));
            match Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(html))
            {
                Ok(resp) => resp,
                Err(_) => text_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            }
        }
        Err(err) => read_error_response(err),
    }
}

/// Serve a non-markdown file raw with a content type derived from its extension.
fn serve_raw(path: &Path, extension: &str) -> Response {
    match std::fs::read(path) {
        Ok(bytes) => match Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type_for(extension))
            .header(header::CONTENT_LENGTH, bytes.len())
            .body(Body::from(bytes))
        {
            Ok(resp) => resp,
            Err(_) => text_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        },
        Err(err) => read_error_response(err),
    }
}

/// Translate an io error into the appropriate HTTP response (404 / 500).
fn read_error_response(err: std::io::Error) -> Response {
    if err.kind() == std::io::ErrorKind::NotFound {
        text_response(StatusCode::NOT_FOUND, "File not found")
    } else {
        text_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
    }
}

/// `GET /view/{*path}` — see module docs.
pub async fn view(State(state): State<ViewerState>, AxumPath(raw): AxumPath<String>) -> Response {
    let decoded = match urlencoding::decode(&raw) {
        Ok(d) => d.into_owned(),
        Err(_) => raw,
    };

    let resolved = match validate_path(state.cwd(), &decoded) {
        Some(path) => path,
        None => return text_response(StatusCode::FORBIDDEN, "Forbidden: Invalid file path"),
    };

    let extension = extension_of(&resolved);
    if extension == "md" || extension == "markdown" {
        serve_markdown(&resolved)
    } else {
        serve_raw(&resolved, &extension)
    }
}
