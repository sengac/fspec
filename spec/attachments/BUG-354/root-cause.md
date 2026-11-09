# BUG-354 root cause

`fspec validate-tags spec/features/fspec-tui-app-bootstrap-rpc009.feature` reports "Work unit ID tag @BUG-299 must be at feature level, not scenario level" for three scenarios. BUG-299 tagged the scenarios it touched instead of the feature. Fix: delete the scenario-level `@BUG-299` lines and add `@BUG-299` to the feature-level tags after `@RPC-009`, so RPC-009 still owns the test file. Coverage is not affected.

## Fix (2026-11-02)
- Red: `fspec validate-tags spec/features/fspec-tui-app-bootstrap-rpc009.feature` reported "Work unit ID tag @BUG-299 must be at feature level, not scenario level" for each scenario-level @BUG-299.
- Fix: removed the scenario-level @BUG-299 tags and added @BUG-299 once at feature level, after @RPC-009, so RPC-009 stays the first work-unit tag and keeps the test file.
- Green: validate-tags on the file passes, `fspec validate` passes, and coverage is unchanged (9/9).
