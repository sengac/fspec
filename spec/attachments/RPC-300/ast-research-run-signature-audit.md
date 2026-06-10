# RPC-300 AST Research — `pub async fn run` signature audit across `codelet/fspec-core/src/commands/`

**Date:** 2026-06-09
**Tool:** AstGrep (Rust language, pattern `pub async fn run($$$ARGS) -> Result<String, FspecCoreError> { $$$BODY }`)
**Scope:** `/home/rquast/projects/fspec/codelet/fspec-core/src/commands/`

## Purpose

Confirm the canonical two-front-doors signature pattern (`args_json: &str, project_root: &Path`) used by all already-ported commands, and verify that `show_coverage.rs` currently matches the stub one-arg pattern (`_args_json: &str`). This audit grounds the Phase B test-writing decision to reproduce the same two-arg async signature in the RPC-300 port.

## Findings

### Ported commands (two-arg signature `args_json: &str, project_root: &Path`)

All confirmed taking the canonical two-arg signature:

- `show_epic.rs:101` ← closest sibling to RPC-300 in style
- `show_feature.rs:118` ← closest sibling to RPC-300 in shape & test layout
- `show_work_unit.rs:68`
- `show_deleted.rs:44`
- `list_work_units.rs:51`
- `list_tags.rs:52`
- `list_prefixes.rs:41` ← reference port from playbook
- `list_features.rs:71`
- `list_epics.rs:41`
- `list_hooks.rs:109`
- `list_attachments.rs:70`
- `list_checkpoints.rs:68`
- `list_virtual_hooks.rs:81`
- `list_feature_tags.rs:100`
- `list_scenario_tags.rs:105`
- `list_schedules.rs:103`
- `list_foundation_sections.rs:144` (_project_root unused but still in signature)
- `query_dependency_stats.rs:117` ← reference for JSON declaration-order pattern
- `query_estimate_accuracy.rs:135`
- `query_metrics.rs:102`
- `query_work_units.rs:48`
- `tag_stats.rs:76`

### Stub commands (one-arg signature `_args_json: &str` only)

These remain stubs returning `FspecCoreError::NotYetPorted` — RPC-300's `show_coverage.rs` is among them:

- `show_coverage.rs:6` ← **target of RPC-300**
- `show_foundation.rs:6`
- `show_event_storm.rs:6`
- `show_foundation_event_storm.rs:6`
- `show_acceptance_criteria.rs:6`
- `show_test_patterns.rs:6`
- (plus ~115 other unrelated stubs)

## Implications for RPC-300

1. **Signature mismatch is the dispatcher contract violator.** The dispatch arm cannot be flipped to `run_ported` until `show_coverage::run` is widened to the two-arg signature, mirroring `show_epic.rs:101` and `show_feature.rs:118`.

2. **Closest reference ports for layout cloning:**
   - `show_feature.rs` — feature-name-resolution + spec/features/ enumeration pattern.
   - `show_epic.rs` — multi-rendering-mode handling (json vs markdown) with declaration-order JSON.
   - `query_dependency_stats.rs` — `#[derive(Serialize)]` + 2-space `to_string_pretty` for byte-stable JSON.
   - `list_features.rs` — `glob_feature_files` enumeration that we may reuse for the project-wide directory scan.

3. **Test-file pattern reference.** `codelet/fspec/tests/cli_show_feature.rs` will be cloned as the structural template for `cli_show_coverage.rs`. `cli_query_dependency_stats.rs` is the secondary reference for in-test JSON fixture authoring.

## How to reproduce

```bash
ast-grep --lang rust \
  --pattern 'pub async fn run($$$ARGS) -> Result<String, FspecCoreError> { $$$BODY }' \
  /home/rquast/projects/fspec/codelet/fspec-core/src/commands/
```

Filter `args_json: &str, project_root: &Path` for ported, `_args_json: &str` for stub.
