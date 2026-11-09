// Feature: spec/features/portable-graph-bundles.feature
//
// Portable Graph Bundles — Export/Import
// Tests that the AST graph can be exported to a portable .astbundle ZIP archive
// and imported back, preserving all nodes, edges, and properties.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::database::GraphDatabase;
use codelet_graph::graph_entities::{entities_to_jsonl, jsonl_to_entities, GraphEntity};
use serde_json::{Map, Value};
use std::io::Read;

/// The AST code schema.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// Helper: create a populated graph for export testing.
async fn setup_export_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-export.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-main-ts","path":"src/main.ts","language":"typescript","lineCount":150,"isTest":false}}
{"type":"File","data":{"slug":"src-utils-py","path":"src/utils.py","language":"python","lineCount":80,"isTest":false}}
{"type":"Function","data":{"slug":"src-main-ts::handleRequest","name":"handleRequest","qualifiedName":"src-main-ts::handleRequest","isAsync":true,"isPublic":true,"paramCount":2,"lineStart":10,"lineEnd":30,"cyclomaticComplexity":4,"parameters":"req, res","source":"async function handleRequest(req, res) { ... }","docstring":"Handle incoming request","decorators":"@Route","language":"typescript"}}
{"type":"Function","data":{"slug":"src-utils-py::parse","name":"parse","qualifiedName":"src-utils-py::parse","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":5,"lineEnd":15,"cyclomaticComplexity":2,"parameters":"data","source":"def parse(data): ...","docstring":"Parse data","decorators":"@staticmethod","language":"python"}}
{"type":"Type","data":{"slug":"src-main-ts::Config","name":"Config","typeKind":"interface","isPublic":true,"lineStart":1,"lineEnd":5,"source":"interface Config { port: number; }","docstring":"App config","decorators":"","language":"typescript"}}
{"type":"Dependency","data":{"slug":"dep-express","name":"express","version":"4.18.2","isDev":false,"source":"npm"}}
{"edge":"Contains","from":"src-main-ts","to":"src-main-ts::handleRequest","data":{"lineStart":10}}
{"edge":"Contains","from":"src-utils-py","to":"src-utils-py::parse","data":{"lineStart":5}}
{"edge":"ContainsType","from":"src-main-ts","to":"src-main-ts::Config","data":{"lineStart":1}}
{"edge":"Imports","from":"src-main-ts","to":"src-utils-py","data":{"importPath":"./utils"}}
{"edge":"DependsOn","from":"src-main-ts","to":"dep-express","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

#[path = "ast_export_import_test/export_creates_valid_astbundle.rs"]
mod export_creates_valid_astbundle;
#[path = "ast_export_import_test/export_import_roundtrip_preserves.rs"]
mod export_import_roundtrip_preserves;

/// Helper: create a tampered bundle with a different schema.pg
fn tamper_bundle_schema(
    source_path: &std::path::Path,
    target_path: &std::path::Path,
    fake_schema: &str,
) {
    let source_file = std::fs::File::open(source_path).expect("open source");
    let mut source_archive = zip::ZipArchive::new(source_file).expect("read source ZIP");

    let target_file = std::fs::File::create(target_path).expect("create target");
    let mut writer = zip::ZipWriter::new(target_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for i in 0..source_archive.len() {
        let mut entry = source_archive.by_index(i).expect("read entry");
        let name = entry.name().to_string();

        if name == "schema.pg" {
            writer.start_file(&name, options).expect("start schema");
            std::io::Write::write_all(&mut writer, fake_schema.as_bytes()).expect("write schema");
        } else {
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).expect("read entry");
            writer.start_file(&name, options).expect("start file");
            std::io::Write::write_all(&mut writer, &contents).expect("write file");
        }
    }
    writer.finish().expect("finish ZIP");
}
