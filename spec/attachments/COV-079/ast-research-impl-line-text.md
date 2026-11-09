# COV-079 AST research (2026-11-07)

```
rust/fspec-core/src/commands/link_coverage.rs:231:13:add_impl_to_row(row, imf, iml)
rust/fspec-core/src/commands/link_coverage/mapping.rs:78:5:add_impl_to_row(tm, impl_file, impl_lines)
rust/fspec-core/src/commands/link_coverage/mapping.rs:126:21:add_impl_to_row(row, impl_file, impl_lines)

rust/fspec-core/src/commands/link_coverage.rs:253:17:add_both_mappings(entry, tf, tl, imf, iml)

rust/fspec-core/src/commands/link_coverage/mapping.rs:181:13:merge_impl(&mut impls, im)

rust/fspec-core/src/commands/link_coverage/mapping.rs:87:18:parse_impl_lines(impl_lines)
rust/fspec-core/src/commands/link_coverage/mapping.rs:132:18:parse_impl_lines(impl_lines)
rust/fspec-core/src/commands/link_coverage/mapping.rs:224:39:parse_impl_lines(range)

No matches found

```

Conclusion: implementation mappings are written in rust/fspec-core/src/commands/link_coverage/mapping.rs only: add_impl_to_row (an impl link onto an existing test row, also used by add_both_mappings) and replace_test_mapping/merge_impl (BUG-502 --replace keeps a row's implMappings, so recorded text travels with them). parse_impl_lines turns "10-24" or "10,11,12" into the stored number list. Recording firstLine/lastLine belongs where the parsed list meets the impl file (add_impl_to_row and the new-row branch of add_both_mappings). The audit reads impl mappings in commands/audit/impl_ranges.rs (BUG-515 range check), where the text comparison and the shift candidate are computed from the same file read. types/coverage.rs ImplMapping keeps unknown fields in `extra`, so typed readers stay compatible.
