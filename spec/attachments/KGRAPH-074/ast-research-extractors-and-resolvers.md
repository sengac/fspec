# AST research: the extractors and resolvers KGRAPH-074/075 replace

fspec astgrep over rust/graph/src/ast_pipeline at 30e2af866. Each line is file:line:col:signature.

## Per-language extractors (one ast-grep pattern extractor per language; replaced by one data-driven fact engine)
```
rust/graph/src/ast_pipeline/ast_c_extractor.rs:35:1:pub fn extract_c(
rust/graph/src/ast_pipeline/ast_cpp_extractor.rs:31:1:pub fn extract_cpp(
rust/graph/src/ast_pipeline/ast_csharp_extractor.rs:39:1:pub fn extract_csharp(
rust/graph/src/ast_pipeline/ast_dart_extractor.rs:89:1:pub fn extract_dart(
rust/graph/src/ast_pipeline/ast_go_extractor.rs:36:1:pub fn extract_go(
rust/graph/src/ast_pipeline/ast_java_extractor.rs:37:1:pub fn extract_java(
rust/graph/src/ast_pipeline/ast_kotlin_extractor.rs:78:1:pub fn extract_kotlin(
rust/graph/src/ast_pipeline/ast_php_extractor.rs:39:1:pub fn extract_php(
rust/graph/src/ast_pipeline/ast_python_extractor.rs:35:1:pub fn extract_python(
rust/graph/src/ast_pipeline/ast_ruby_extractor.rs:41:1:pub fn extract_ruby(
rust/graph/src/ast_pipeline/ast_rust_extractor.rs:54:1:pub fn extract_rust(
rust/graph/src/ast_pipeline/ast_scala_extractor.rs:102:1:pub fn extract_scala(
rust/graph/src/ast_pipeline/ast_swift_extractor.rs:39:1:pub fn extract_swift(
rust/graph/src/ast_pipeline/ast_ts_extractor.rs:50:1:pub fn extract_typescript(
rust/graph/src/ast_pipeline/cargo_dep_extractor.rs:16:1:pub fn extract_cargo_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/composer_dep_extractor.rs:13:1:pub fn extract_composer_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/csproj_dep_extractor.rs:11:1:pub fn extract_csproj_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/gemfile_dep_extractor.rs:11:1:pub fn extract_gemfile_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/gomod_dep_extractor.rs:11:1:pub fn extract_go_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/java_dep_extractor.rs:12:1:pub fn extract_java_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/npm_dep_extractor.rs:17:1:pub fn extract_npm_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/pip_dep_extractor.rs:14:1:pub fn extract_python_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/pubspec_dep_extractor.rs:19:1:pub fn extract_pubspec_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/sbt_dep_extractor.rs:11:1:pub fn extract_sbt_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
rust/graph/src/ast_pipeline/swift_dep_extractor.rs:12:1:pub fn extract_swift_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
```

## Lexical call and type-name scanning, per-file resolution (B08; replaced by call-site facts and the global resolve pass)
```

rust/graph/src/ast_pipeline/edge_helpers.rs:18:1:pub fn build_import_edge(
rust/graph/src/ast_pipeline/edge_helpers.rs:188:1:pub fn extract_constructor_names_from_body(body: &str, out: &mut HashSet<String>) {
rust/graph/src/ast_pipeline/edge_helpers.rs:227:1:pub fn extract_member_call_names(body: &str, out: &mut HashSet<String>) {
rust/graph/src/ast_pipeline/edge_helpers.rs:265:1:pub fn resolve_calls(
rust/graph/src/ast_pipeline/edge_helpers.rs:311:1:pub fn resolve_type_refs(
rust/graph/src/ast_pipeline/edge_helpers.rs:53:1:pub fn build_calls_edge(from_fn_slug: &str, to_fn_slug: &str, entities: &mut Vec<GraphEntity>) {
rust/graph/src/ast_pipeline/edge_helpers.rs:63:1:pub fn build_typeref_edge(from_fn_slug: &str, to_type_slug: &str, entities: &mut Vec<GraphEntity>) {
rust/graph/src/ast_pipeline/edge_helpers.rs:78:1:pub fn extract_call_names_from_body(body: &str, out: &mut HashSet<String>) {
```

## Per-language import resolvers against known_files (B09; replaced by exact module rules in the resolve pass)
```
rust/graph/src/ast_pipeline/ast_java_extractor.rs:278:1:fn resolve_java_import(import_path: &str, known_files: &HashSet<String>) -> Option<String> {
rust/graph/src/ast_pipeline/ast_python_extractor.rs:283:1:fn resolve_python_module(module_path: &str, known_files: &HashSet<String>) -> Option<String> {
rust/graph/src/ast_pipeline/ast_rust_extractor.rs:295:1:fn resolve_rust_module_path(module_path: &str, known_files: &HashSet<String>) -> Option<String> {
```

## Entity identity (B04/B05)
```
rust/graph/src/ast_pipeline/helpers.rs:15:1:pub fn slugify_path(path: &str) -> String {

rust/graph/src/ast_pipeline/ast_csharp_extractor.rs:114:27:format!("{file_slug}::{name}")
rust/graph/src/ast_pipeline/ast_csharp_extractor.rs:177:29:format!("{file_slug}::{name}")
rust/graph/src/ast_pipeline/ast_rust_extractor.rs:135:27:format!("{file_slug}::{name}")
rust/graph/src/ast_pipeline/ast_rust_extractor.rs:183:29:format!("{file_slug}::{name}")
rust/graph/src/ast_pipeline/ast_c_extractor.rs:113:27:format!("{file_slug}::{name}")
```
