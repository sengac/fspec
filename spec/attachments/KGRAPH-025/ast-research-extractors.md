# AST Research: Multi-Language Extractor Functions

## AST Extractors (13 languages)
All follow the signature: `pub fn extract_<lang>(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String>`

| Language | File | Function |
|----------|------|----------|
| TypeScript/JS | ast_ts_extractor.rs:47 | `extract_typescript` |
| Rust | ast_rust_extractor.rs:41 | `extract_rust` |
| Python | ast_python_extractor.rs:27 | `extract_python` |
| Go | ast_go_extractor.rs:29 | `extract_go` |
| Java | ast_java_extractor.rs:35 | `extract_java` |
| C | ast_c_extractor.rs:29 | `extract_c` |
| C++ | ast_cpp_extractor.rs:26 | `extract_cpp` |
| C# | ast_csharp_extractor.rs:29 | `extract_csharp` |
| Ruby | ast_ruby_extractor.rs:28 | `extract_ruby` |
| Kotlin | ast_kotlin_extractor.rs:35 | `extract_kotlin` |
| Swift | ast_swift_extractor.rs:28 | `extract_swift` |
| Scala | ast_scala_extractor.rs:33 | `extract_scala` |
| PHP | ast_php_extractor.rs:34 | `extract_php` |

## Dependency Extractors (10 package managers)
All follow: `pub fn extract_<pm>_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String>`

| Package Manager | File | Source Field |
|----------------|------|--------------|
| npm | npm_dep_extractor.rs:17 | "npm" |
| cargo | cargo_dep_extractor.rs:16 | "cargo" |
| pip | pip_dep_extractor.rs:14 | "pip" |
| go | gomod_dep_extractor.rs:11 | "go" |
| maven/gradle | java_dep_extractor.rs:12 | "maven"/"gradle" |
| composer | composer_dep_extractor.rs:13 | "composer" |
| gem | gemfile_dep_extractor.rs:11 | "gem" |
| nuget | csproj_dep_extractor.rs:11 | "nuget" |
| sbt | sbt_dep_extractor.rs:11 | "sbt" |
| spm | swift_dep_extractor.rs:12 | "spm" |
