# BUG-260: Codex refusal-history follow-up failure

## Finding and provenance

Reproduced at `f0eb0759`; dedicated native history introduced by `2699693b`.
Refusal output is projected into visible assistant text, but native refusal
items are not normalized to that readable representation for lineage matching.
The next user request, nudge or tool-result continuation can fail before inference.

## Source evidence

- `rust/providers/src/codex/answers.rs:44`: refusal and output_text become text.
- `rust/providers/src/codex/history_projection.rs:11`: recognizes only output_text.
- `rust/providers/src/codex/history_lineage.rs:83`: rejects unmatched ancestry.
- `rust/patches/rig-core/src/providers/openai/responses_api/mod.rs:448`: serializes
  assistant text back as output_text rather than native refusal.
- `rust/providers/tests/prov148_codex_completed_answers.rs:8`: display-only coverage.

## Minimal reproduction against production APIs

```rust
use codelet_providers::codex::history::CodexHistory;
use serde_json::{json, Value};

fn follow_up(part: Value) -> Result<(), Box<dyn std::error::Error>> {
    let history = CodexHistory::default();
    let user = json!({"type":"message", "role":"user", "content":"hello"});
    let request = json!({"model":"audit-model", "input":[user.clone()]});
    history.commit(&request, &[json!({
        "type":"message", "id":"answer", "role":"assistant", "content":[part]
    })])?;
    let mut next = json!({"model":"audit-model", "input":[
        user,
        {"type":"message", "role":"assistant", "content":[
            {"type":"output_text", "text":"No"}
        ]},
        {"type":"message", "role":"user", "content":"Continue"}
    ]});
    history.prepare(&mut next)?;
    Ok(())
}
```

Observed: `follow_up(json!({"type":"output_text","text":"No"}))` succeeds.
Replacing the part with `{"type":"refusal","refusal":"No"}` fails with
assistant/tool-result/DAG ancestry. Both diagnostic assertions passed in a
temporary test executable linked to the compiled production provider.

## Acceptance/test direction

- Start with a failing integration test of a real serialized follow-up request.
- Cover refusal-only and mixed text/refusal sections, tool-result follow-up,
  outer nudge and subsequent user input, with reasoning preservation on/off.
- Retain native refusal items and exact opaque reasoning in request history;
  normalize only the readable comparison, not the trusted checkpoint itself.
- Keep genuine ancestry mismatches rejected and do not change refusal semantics.

Existing contracts: PROV-148, PROV-149, CMPCT-078, CONT-002/005.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
