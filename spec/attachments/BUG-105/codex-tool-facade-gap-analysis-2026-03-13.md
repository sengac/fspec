# BUG-105: Codex Tool Facade Gap Analysis

Date: 2026-03-13
Work Unit: BUG-105
Target file reviewed: `codelet/tools/src/facade/codex.rs`
Comparison baseline: Codex CLI tool spec snapshot in `spec/attachments/TOOL-015/codex-tool-facade-research.md`

## Summary

The current Codex facade includes the core renamed tools (`shell_command`, `read_file`, `list_dir`, `grep_files`) but is missing multiple Codex-native tool calls and includes a non-native `glob` tool. This likely contributes to Codex sessions degrading editing flows into shell-based patch attempts instead of first-class patch/edit tool calls.

## Correctly Named in `codelet/tools/src/facade/codex.rs`

- `shell_command` (`codelet/tools/src/facade/codex.rs:78`)
- `read_file` (`codelet/tools/src/facade/codex.rs:132`)
- `list_dir` (`codelet/tools/src/facade/codex.rs:196`)
- `grep_files` (`codelet/tools/src/facade/codex.rs:248`)

## Incorrect/Non-native Tool Name Present

- `glob` is exposed (`codelet/tools/src/facade/codex.rs:304`), but Codex native function-tool set does not define `glob` in the referenced CLI tool spec.

## Missing Codex-Native Tools (Expected but Not Present in `codex.rs`)

- `apply_patch`
- `view_image`
- `update_plan`
- `shell`
- `exec_command`
- `write_stdin`
- `request_user_input`
- native `web_search` tool type handling

No corresponding tool facade definitions were found in `codelet/tools/src/facade/codex.rs` for the above names.

## Interface Coverage Gaps for Existing Facades

### `shell_command`

Expected Codex schema includes additional fields beyond current facade support, including:
- `login`
- `sandbox_permissions`
- `justification`
- `prefix_rule`

Current mapping in `codelet/tools/src/facade/codex.rs` only maps `command` into internal execution params (`codelet/tools/src/facade/codex.rs:107`).

### `read_file`

Expected Codex schema includes:
- `mode`
- `indentation` object (anchor_line, max_levels, include_siblings, include_header, max_lines)

Current facade schema/mapping only covers `file_path`, `offset`, `limit` (`codelet/tools/src/facade/codex.rs:142`, `codelet/tools/src/facade/codex.rs:163`).

### `list_dir`

Expected schema includes `offset` and `limit` as optional pagination controls.

Current facade includes only `dir_path` and `depth` in schema (`codelet/tools/src/facade/codex.rs:206`), and maps only `dir_path` (`codelet/tools/src/facade/codex.rs:223`).

### `grep_files`

Schema exposes `include` and `limit`, but mapping currently only consumes:
- `pattern`
- `path`

Mapping location: `codelet/tools/src/facade/codex.rs:281`.

## Relevant Reference Source

Codex CLI tool naming/schema snapshot used for expected names:
- `spec/attachments/TOOL-015/codex-tool-facade-research.md:9`

Key expected tools in that artifact:
- `apply_patch` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:60`)
- `view_image` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:76`)
- `update_plan` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:83`)
- `web_search` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:91`)
- `shell` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:97`)
- `exec_command` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:107`)
- `write_stdin` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:121`)
- `request_user_input` (`spec/attachments/TOOL-015/codex-tool-facade-research.md:131`)

## Impact on BUG-105

Given BUG-105 describes Codex patch/edit operations falling back to shell behavior, the absence of explicit `apply_patch`-style capability (or equivalent first-class edit path selection) in the Codex facade aligns with the observed symptom.
