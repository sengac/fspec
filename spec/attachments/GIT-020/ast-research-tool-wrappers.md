# AST Research: Tool Wrappers for GIT-020

## Research Summary

Analysis of file operation tool wrappers to understand how to integrate effective_cwd for isolated session support.

## Findings

### 1. File Tool Facade Wrappers with Session ID

Found at `codelet/tools/src/facade/wrapper.rs`:

```
codelet/tools/src/facade/wrapper.rs:220:5:pub fn new(facade: BoxedFileToolFacade, session_id: Uuid) -> Self {
codelet/tools/src/facade/wrapper.rs:480:5:pub fn new(facade: BoxedFspecToolFacade, session_id: Uuid) -> Self {
codelet/tools/src/facade/wrapper.rs:588:5:pub fn new(facade: BoxedBashToolFacade, session_id: Uuid) -> Self {
codelet/tools/src/facade/wrapper.rs:913:5:pub fn new(facade: BoxedBridgeToolFacade, session_id: Uuid) -> Self {
```

Key insight: FileToolFacadeWrapper and BashToolFacadeWrapper already have `session_id` from TOOL-012 pattern.

### 2. effective_cwd Method Locations

```
codelet/napi/src/session_manager.rs:1028:5:pub fn effective_cwd(&self) -> PathBuf {
codelet/git/src/isolated_session.rs:83:5:pub fn effective_cwd(&self) -> PathBuf {
```

BackgroundSession has effective_cwd method that returns worktree_path if present, else project root.

### 3. Existing Callback Pattern

From wrapper.rs (lines 391-425), there's already a callback pattern for getting work unit stage:

```rust
pub type GetWorkUnitStageCallback = fn(String) -> Option<String>;
static GET_WORK_UNIT_STAGE_CALLBACK: std::sync::OnceLock<GetWorkUnitStageCallback> = ...;
pub fn set_get_work_unit_stage_callback(callback: GetWorkUnitStageCallback) { ... }
fn get_work_unit_stage(session_id: Uuid) -> Option<String> { ... }
```

### 4. Tool Call Methods

```
codelet/tools/src/facade/wrapper.rs:74:5:async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
codelet/tools/src/facade/wrapper.rs:264:5:async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
```

FileToolFacadeWrapper.call() at line 264 is where we need to resolve paths using effective_cwd.

## Implementation Plan

1. Add `GetEffectiveCwdCallback` type similar to `GetWorkUnitStageCallback`
2. Add global `GET_EFFECTIVE_CWD_CALLBACK` OnceLock
3. Add `set_get_effective_cwd_callback()` and `get_effective_cwd()` functions
4. Modify `FileToolFacadeWrapper.call()` to resolve paths using effective_cwd
5. Modify `BashToolFacadeWrapper.call()` to use effective_cwd for cwd
6. Register callback in codelet-napi initialization (similar to stage callback)
