# BUG-372 root cause

`rust/test-helpers/src/source_composition.rs`:

```rust
pub fn read_tree_or_file(path: impl AsRef<Path>) -> io::Result<String> {
    let path = path.as_ref();
    if path.extension().is_some_and(|ext| ext == "rs") {
        if let Ok(text) = modules::read_tree(path) {
            return Ok(text);
        }
    }
    std::fs::read_to_string(path)
}
```

When `modules::read_tree` fails (a missing child module file, a bad `#[path]`, or a parse error), the error is dropped and the function returns only the single file. Source-shape tests then assert against less text than the module really has, and nobody is told the tree was broken. The silent difference between "tree" and "file" also caused BUG-371, where a test wanted physical lines and got the tree.

Fix direction: return the `read_tree` error for `.rs` paths, and give callers explicit `read_module_tree` (which already exists) and `read_file` functions so each test states what it measures.
