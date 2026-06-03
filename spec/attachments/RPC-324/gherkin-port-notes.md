# RPC-324 — `validate-tags` — Gherkin Port Notes

**Category:** (A) Parse/Validate
**Master guide:** `spec/attachments/RPC-003/gherkin-porting-guide.md`

## Current TS Behavior
- Delegates per-file work to `validate-tags-file.ts` which imports `@cucumber/gherkin` + `@cucumber/messages` and uses `work-unit-tags.loadWorkUnitsData`.
- For every `.feature` file: parses, walks tags on feature + scenarios + examples, cross-references the tag registry (`spec/tags.json`) and existing work units.

## Rust Port Plan
- Parse each feature with `gherkin::Feature::parse_path`.
- Iterate `feat.tags`, `feat.scenarios[].tags`, `feat.examples[].tags`, `feat.rules[].tags` to enumerate all tags in the file.
- Tags are stored **without** the `@` prefix (master guide §3 + §7). Re-add `@` when reporting violations to match TS output exactly.
- Load tag registry and work-unit data (separate non-Gherkin modules); validate every tag is either registered OR is a work-unit ID tag (delegate to `gherkin_tags::is_work_unit_tag`).

## Key Files
- TS source: `src/commands/validate-tags.ts`, `src/commands/validate-tags-file.ts`
- Shared modules: `gherkin_tags.rs`, `gherkin_query.rs`

## Gotchas
- `Examples` blocks can have their own tags — don't forget to scan them.
- `Background` has NO tags (parser & spec correct).
- Rule-level tags exist in `feat.rules[].tags` — TS code may or may not check them; verify against TS behavior before porting.
