# Reasoning discovery

AST query: `pub fn event(&mut self, kind: &str, event: &Value) -> Result<Vec<Choice>, CompletionError> { $$$BODY }`
matches `rust/providers/src/codex/reasoning.rs:40`.

`Reasoning::event` recognizes raw and summary delta/done events. `Sections`
deduplicates by kind, item identity and section index. Codex streaming emits
`Choice::ReasoningDelta`, which the CLI consumes as thinking. The ordinary
Responses parser is not modified. Separate features cover transport normalization,
completed sections, optional identities, and the production CLI bridge.

Prior failing evidence: `/tmp/fspec-prov150-red.log` and
`/tmp/fspec-prov150-sections-red.log`. These are inherited test-first changes;
traceability repairs do not claim to be newly introduced behavioral changes.
