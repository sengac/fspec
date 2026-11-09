//! KGRAPH-077: the query index of a code graph version.
//!
//! The forward and reverse Calls adjacency, the metadata of every function
//! and type without source text, every file, and the file of every function
//! and type. [`GraphDatabase::query_index`] builds it with a fixed number of
//! whole-graph queries once per graph version, so no query runs once per
//! function. Every map is keyed by slug and every adjacency list is sorted
//! by slug, so whatever is read from the index comes out in the same order
//! in every process.

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use crate::ast_dispatch::AST_QUERIES;
use crate::database::GraphDatabase;

/// The query index of one graph version.
pub struct QueryIndex {
    /// The graph version (manifest db_version) the index was built from.
    pub version: u64,
    /// File rows (slug, path, language, lineCount, isTest) by slug.
    pub files: BTreeMap<String, Value>,
    /// Function rows without `source`, by slug.
    pub functions: BTreeMap<String, Value>,
    /// Type rows without `source`, by slug.
    pub types: BTreeMap<String, Value>,
    /// The file slug of every function (its Contains edge).
    pub function_file: HashMap<String, String>,
    /// The file slug of every type (its ContainsType edge).
    pub type_file: HashMap<String, String>,
    /// Callees of each function, sorted by slug.
    pub callees: HashMap<String, Vec<String>>,
    /// Callers of each function, sorted by slug.
    pub callers: HashMap<String, Vec<String>>,
    /// The properties (callCount, isConditional) of each Calls edge by
    /// (caller, callee), read from the edge rows (KGRAPH-078).
    pub call_edges: HashMap<(String, String), Value>,
}

/// Run a query that must return rows.
pub(crate) async fn rows(
    db: &GraphDatabase,
    query_name: &str,
    params: Option<&Value>,
) -> Result<Vec<Value>, String> {
    match db
        .query_with_source(AST_QUERIES, query_name, params)
        .await?
    {
        Value::Array(rows) => Ok(rows),
        other => Err(format!(
            "Graph query '{query_name}' returned no rows but {other}"
        )),
    }
}

fn field<'a>(row: &'a Value, name: &str, query_name: &str) -> Result<&'a str, String> {
    row.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Graph query '{query_name}' returned a row without {name}: {row}"))
}

/// Rows keyed by their `slug`.
fn by_slug(rows: Vec<Value>, query_name: &str) -> Result<BTreeMap<String, Value>, String> {
    rows.into_iter()
        .map(|row| Ok((field(&row, "slug", query_name)?.to_string(), row)))
        .collect()
}

/// (from, to) pairs as a map from each `from` to its sorted `to`s.
fn pairs(
    rows: &[Value],
    from: &str,
    to: &str,
    query_name: &str,
) -> Result<Vec<(String, String)>, String> {
    rows.iter()
        .map(|row| {
            Ok((
                field(row, from, query_name)?.to_string(),
                field(row, to, query_name)?.to_string(),
            ))
        })
        .collect()
}

fn sorted_adjacency(edges: impl Iterator<Item = (String, String)>) -> HashMap<String, Vec<String>> {
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for (from, to) in edges {
        adjacency.entry(from).or_default().push(to);
    }
    for list in adjacency.values_mut() {
        list.sort();
        list.dedup();
    }
    adjacency
}

impl QueryIndex {
    /// Build the index of `db`'s graph at `version`.
    pub(crate) async fn load(db: &GraphDatabase, version: u64) -> Result<Self, String> {
        let calls = pairs(
            &rows(db, "all_calls", None).await?,
            "caller",
            "callee",
            "all_calls",
        )?;
        let functions = by_slug(rows(db, "function_index", None).await?, "function_index")?;
        let types = by_slug(rows(db, "type_index", None).await?, "type_index")?;
        let files = by_slug(rows(db, "all_files", None).await?, "all_files")?;
        let function_file = pairs(
            &rows(db, "all_contains", None).await?,
            "function",
            "file",
            "all_contains",
        )?
        .into_iter()
        .collect();
        let type_file = pairs(
            &rows(db, "all_contains_type", None).await?,
            "type",
            "file",
            "all_contains_type",
        )?
        .into_iter()
        .collect();
        let mut call_edges = HashMap::new();
        for row in db.edge_rows("Calls").await? {
            let from = field(&row, "from", "Calls edge rows")?.to_string();
            let to = field(&row, "to", "Calls edge rows")?.to_string();
            call_edges.insert((from, to), row.get("data").cloned().unwrap_or(Value::Null));
        }
        Ok(Self {
            version,
            call_edges,
            files,
            functions,
            types,
            function_file,
            type_file,
            callees: sorted_adjacency(calls.iter().cloned()),
            callers: sorted_adjacency(calls.into_iter().map(|(from, to)| (to, from))),
        })
    }

    /// The path of the file a function is in.
    pub fn function_path(&self, slug: &str) -> Option<&str> {
        self.file_path(self.function_file.get(slug)?)
    }

    /// The path of the file a type is in.
    pub fn type_path(&self, slug: &str) -> Option<&str> {
        self.file_path(self.type_file.get(slug)?)
    }

    fn file_path(&self, file_slug: &str) -> Option<&str> {
        self.files.get(file_slug)?.get("path")?.as_str()
    }

    /// Whether the file a function or type is in is a test file.
    pub fn in_test_file(&self, file_slug: Option<&String>) -> bool {
        file_slug
            .and_then(|slug| self.files.get(slug))
            .and_then(|file| file.get("isTest"))
            .and_then(Value::as_bool)
            == Some(true)
    }
}

/// The source text of a function or type row: the row a response returns,
/// with its `source` fetched by one query.
pub(crate) async fn with_source(
    db: &GraphDatabase,
    source_query: &str,
    slug: &str,
    mut row: Value,
) -> Result<Value, String> {
    let params = serde_json::json!({ "slug": slug });
    let found = rows(db, source_query, Some(&params)).await?;
    let source = match found.as_slice() {
        [one] => one.get("source").cloned().unwrap_or(Value::Null),
        _ => {
            return Err(format!(
                "Graph query '{source_query}' found {} rows for {slug}",
                found.len()
            ))
        }
    };
    if let Value::Object(map) = &mut row {
        map.insert("source".to_string(), source);
    }
    Ok(row)
}
