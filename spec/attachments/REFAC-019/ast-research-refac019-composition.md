# REFAC-019 AST and composition discovery

AstGrep(language=rust, path=rust/rpc/src/lib.rs,
pattern=`pub trait FspecService { $$$MEMBERS }`) returned:

```text
/home/rquast/projects/fspec/rust/rpc/src/lib.rs:68:1:pub trait FspecService {
```

Read-only independent review confirmed whole-item accumulator macros are viable
provided the final macro emits the full attributed item, original member order is
retained and expansion stays in the original logical module. Existing embedded
OAuth and WebSocket focus-event methods use super paths, so merely moving the
trait impl into a new child module would change name resolution. Generated tarpc
types must remain at the root. Defaults and bodies must be pinned, not signatures
alone. Contract resolution must follow literal includes and the real macro chain,
rejecting unused definitions, cycles and dropped accumulator tokens.

The current rpc/build.rs hashes only src/lib.rs and top-level rpc-types .rs files.
A nested declaration edit is invisible today. The current mcp_coverage test splits
the literal root trait text and must resolve actual composition without weakening
its exact method equality or strict-schema checks. Existing source-shape readers
must inspect reconstructed reachable items, not broad directories or dead macros.

No production edit was made during this discovery. No BUG-283-owned file was read
for the independent composition review. Behavioral and ordered token-contract
baselines will be captured before any production decomposition.
