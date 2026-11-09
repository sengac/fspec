# REFAC-034 design: split generate-compaction-tool.feature

## Problem

`spec/features/generate-compaction-tool.feature` is 300 lines. BUG-345 must tag the feature that
owns the provider tool-chain scenario (`GenerateCompaction tool wired into provider agent
builders`) with `@BUG-345` and note the moved OpenAI file, which would take the file over the
300-line limit. The file is split first.

## Chosen split

The three wiring scenarios move, unchanged, to a new feature
`spec/features/generate-compaction-tool-wiring.feature`:

- Compactor sub-agent provider arms match the DeepSearch clone (CMPCT-046c, rule 22)
- GenerateCompaction tool wired into provider agent builders (rule 13)
- Handler registration lifecycle mirrors the DeepSearch pattern (rule 13)

The new feature's first work-unit tag is `@CMPCT-045`, the owner of the
`cmpct045_generate_compaction_behavioral` test target that holds all three tests, followed by
`@CMPCT-046` and `@REFAC-034`. Its doc string restates rules 13 and 22 and points at the parent
feature. The parent's example-mapping comment block stays whole, since it records the CMPCT-045 example map. The misplaced
"Tool surface and registration lifecycle" section banner (it sat inside the wiring scenario,
between the title and the first step) is dropped rather than moved. The provider-arms scenario's
tag block lists `@context-management @session @regression` twice; it moves with each tag once.
Every other scenario, step and tag in the parent is unchanged.

Coverage for the three scenarios moves to the new feature's coverage file with the same test
mappings. The two scenarios that had no implementation mapping gain one while being relinked
(compaction_builder.rs provider match; agent_loop/services.rs registration and
terminal_overflow_recovery.rs CompactorRegistration drop).

## Guard

Scenario `The GenerateCompaction feature files stay under 300 lines` in the new feature
`spec/features/generate-compaction-feature-split.feature`, tested by
`rust/agent-loop/tests/refac034_generate_compaction_feature_split.rs`: both feature files are
under 300 lines and the three wiring scenarios live in the wiring feature, not the parent.

## Alternatives rejected

- Trimming the generated example-mapping comment block: it is not a split and loses the rule text
  that sits beside the scenarios.
- Tagging only the scenario: the bug still has to be tagged on a feature, and the file would still
  grow past 300 lines.

## Wire / API changes

None.
