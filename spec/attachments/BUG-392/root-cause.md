# BUG-392 root cause

`codelet_test_helpers::source_composition::read_tree_or_file` (rust/test-helpers/src/source_composition.rs):

```rust
if path.extension().is_some_and(|ext| ext == "rs") {
    if let Ok(text) = modules::read_tree(path) {
        return Ok(text);
    }
}
std::fs::read_to_string(path)
```

Any composition error (a child module file that does not exist, a `#[path]` that does not resolve, a syn parse
failure) is dropped and the bare file text is returned. Size-ceiling tests such as
`source_shape_rpc054::every_file_under_provider_settings_stays_under_300_lines` then measure less than the composed
module and pass or fail for the wrong reason with no message. Found during BUG-388/BUG-389.

Fix direction: return the composition error (or only fall back for an explicit, named case).
