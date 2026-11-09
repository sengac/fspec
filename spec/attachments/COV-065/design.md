# COV-065 design notes

## Finding

`fspec validate-tags` (2026-11-02, after 79bb9655) reports 539 feature files with violations:

| Violation | Count |
|---|---|
| Missing required feature-group tag | 424 |
| Missing required component tag | 311 |
| Work-unit ID tag at scenario level | 282 |
| Unregistered tags | rest (60 in rpc054-provider-settings-view.feature, 32 in provider-settings-screen.feature, 27 in unified-exec-tool.feature, ...) |

Example: `spec/features/generate-compaction-tool.feature` carries `@BUG-311` on three scenarios
(INCREMENTAL prompt, Unknown target session, Tool result is the queued DAG candidate).

## Approach

Work by area, one card per area if it is large. For each feature: register real tags with
`fspec register-tag` or replace them with registered ones; move work-unit ID tags to feature
level; add the missing component and feature-group tags. A feature that would pass 300 lines
is split first under its own REFAC card. The work is done when `fspec validate-tags` passes.
