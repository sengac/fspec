// Feature: spec/features/variable-symbol-tracking.feature
//
// Variable and Symbol Tracking
// Tests that Variable nodes are extracted during indexing and searchable
// via ast_search with entity_type=Variable. Verifies module-level and
// class-level variable extraction across TypeScript, Python, Rust, Java,
// and that function-local variables are excluded.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::database::GraphDatabase;

/// The AST code schema (must include Variable node + ContainsVariable edge).
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// Bundled AST query source for named queries.
const AST_QUERIES: &str = include_str!("../schemas/ast-queries.gq");

/// Helper: create a graph with Variable nodes for testing search.
async fn setup_variable_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-variables.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-config-ts","path":"src/config.ts","language":"typescript","lineCount":20,"isTest":false}}
{"type":"File","data":{"slug":"src-utils-py","path":"src/utils.py","language":"python","lineCount":50,"isTest":false}}
{"type":"File","data":{"slug":"src-main-rs","path":"src/main.rs","language":"rust","lineCount":100,"isTest":false}}
{"type":"File","data":{"slug":"src-app-java","path":"src/App.java","language":"java","lineCount":80,"isTest":false}}
{"type":"Variable","data":{"slug":"src-config-ts::API_KEY","name":"API_KEY","path":"src/config.ts","lineStart":1,"value":"'abc123'","scope":"module","scopeName":"","isConstant":true,"language":"typescript"}}
{"type":"Variable","data":{"slug":"src-config-ts::PORT","name":"PORT","path":"src/config.ts","lineStart":2,"value":"8080","scope":"module","scopeName":"","isConstant":true,"language":"typescript"}}
{"type":"Variable","data":{"slug":"src-utils-py::MAX_RETRIES","name":"MAX_RETRIES","path":"src/utils.py","lineStart":3,"value":"3","scope":"module","scopeName":"","isConstant":true,"language":"python"}}
{"type":"Variable","data":{"slug":"src-utils-py::logger","name":"logger","path":"src/utils.py","lineStart":5,"value":"logging.getLogger(__name__)","scope":"module","scopeName":"","isConstant":false,"language":"python"}}
{"type":"Variable","data":{"slug":"src-main-rs::MAX_SIZE","name":"MAX_SIZE","path":"src/main.rs","lineStart":10,"value":"1024","scope":"module","scopeName":"","isConstant":true,"language":"rust"}}
{"type":"Variable","data":{"slug":"src-main-rs::INSTANCE","name":"INSTANCE","path":"src/main.rs","lineStart":12,"value":"Lazy::new(|| Db::connect())","scope":"module","scopeName":"","isConstant":false,"language":"rust"}}
{"type":"Variable","data":{"slug":"src-app-java::App.DB_URL","name":"DB_URL","path":"src/App.java","lineStart":8,"value":"\"jdbc:postgresql://localhost/db\"","scope":"class","scopeName":"App","isConstant":true,"language":"java"}}
{"type":"Variable","data":{"slug":"src-app-java::App.MAX_CONNECTIONS","name":"MAX_CONNECTIONS","path":"src/App.java","lineStart":9,"value":"10","scope":"class","scopeName":"App","isConstant":true,"language":"java"}}
{"type":"Variable","data":{"slug":"src-config-ts::API_URL","name":"API_URL","path":"src/config.ts","lineStart":3,"value":"'https://api.example.com'","scope":"module","scopeName":"","isConstant":true,"language":"typescript"}}
{"edge":"ContainsVariable","from":"src-config-ts","to":"src-config-ts::API_KEY","data":{}}
{"edge":"ContainsVariable","from":"src-config-ts","to":"src-config-ts::PORT","data":{}}
{"edge":"ContainsVariable","from":"src-config-ts","to":"src-config-ts::API_URL","data":{}}
{"edge":"ContainsVariable","from":"src-utils-py","to":"src-utils-py::MAX_RETRIES","data":{}}
{"edge":"ContainsVariable","from":"src-utils-py","to":"src-utils-py::logger","data":{}}
{"edge":"ContainsVariable","from":"src-main-rs","to":"src-main-rs::MAX_SIZE","data":{}}
{"edge":"ContainsVariable","from":"src-main-rs","to":"src-main-rs::INSTANCE","data":{}}
{"edge":"ContainsVariable","from":"src-app-java","to":"src-app-java::App.DB_URL","data":{}}
{"edge":"ContainsVariable","from":"src-app-java","to":"src-app-java::App.MAX_CONNECTIONS","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

#[path = "ast_variable_tracking_test/ast_stats_includes_variable.rs"]
mod ast_stats_includes_variable;
#[path = "ast_variable_tracking_test/typescript_const_declarations_extracted.rs"]
mod typescript_const_declarations_extracted;
