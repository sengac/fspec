# REFAC-033 design: split the cmpct045 behavioural test file

## Problem

`rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs` is 1689 lines. BUG-345 must fix
one of its tests (`generate_compaction_tool_wired_into_every_provider_tool_chain` still reads
rust/providers/src/openai.rs, whose agent wiring REFAC-031 moved to openai/agent.rs). AGENTS.md
requires a touched file over 300 lines to be split first.

## Chosen split

The target keeps its inner attributes, `mod selected`, imports and helpers' context, and
`include!`s, in order, fragments `tests/cmpct045/part_NN.rs`, cut by a script only at blank lines
between top-level items (no test or helper is split). `include!` keeps every item in the crate root
of the test target, so names, visibility, `use` imports and `#[serial]` attributes are unchanged.
A script checks that the reassembled file's non-blank lines equal the original, except the one `include_str!("fixtures/..")` path, which becomes `"../fixtures/.."` because `include_str!` resolves relative to the fragment. Coverage mappings
into the moved lines are relocated by the script's line map.

## Alternatives rejected

- `#[path]` submodules: every fragment would need its own `use super::*` and the shared helpers
  `pub(super)`; a larger diff for a no-behaviour refactor.
- Editing BUG-345's one line without splitting: violates the 300-line rule.

## Guard

`spec/features/compaction-behavioural-test-split.feature` and
`rust/agent-loop/tests/refac033_cmpct045_split.rs` (every file under 300 lines).

## Wire / API changes

None.
