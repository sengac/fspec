# PROV-115 — AST research: compaction-threshold save path

AST queries (AstGrep, language=rust) run against the live Rust port to confirm
the exact functions on the profile save path and the shared parser the design
constraint warns about. Complements the verified parity doc
`compaction-threshold-validation-parity.md`.

## Query 1 — the shared parser (NO range checks today)

Pattern: `pub fn parse_compaction_trigger($$$ARGS) -> $RET { $$$BODY }`

Hit:
- `codelet/fspec-tui/src/views/model_selector/form.rs:222`
  `pub fn parse_compaction_trigger(raw: &str) -> (Option<String>, Option<u32>)`

Body (lines 222-237) splits on a trailing `%` → `("percentage", n)` or a bare
integer → `("tokens", n)`; otherwise `(None, None)`. There is **no** `1..=100`
percentage range guard and **no** `>= 1000` token minimum. This is the shared
helper consumed by BOTH the model_selector custom-model form and the profile
form, so per the design constraint it must stay untouched.

## Query 2 — the profile save path

Pattern: `pub fn build_definition(&self) -> $RET { $$$BODY }`

Hit:
- `codelet/fspec-tui/src/views/provider_settings/profile_form.rs:145`
  `pub fn build_definition(&self) -> Option<ProfileDefinition>`

`build_definition` calls `parse_compaction_trigger(&self.compaction_threshold)`
(line 149-150) and folds the returned split fields straight into the
`ProfileDefinition` (lines 156-157) with no range enforcement. This is the
single profile-form-only seam where the PROV-115 range guard must be applied
(after the shared splitter returns), so the model_selector path is unaffected.

## Conclusion (matches the parity doc)

- Fix seam: `profile_form::build_definition` (profile-form-only).
- Do NOT modify `model_selector/form.rs::parse_compaction_trigger`.
- Range constants to mirror from TS: MIN_PERCENTAGE=1, MAX_PERCENTAGE=100,
  MIN_TOKEN_THRESHOLD=1000.
