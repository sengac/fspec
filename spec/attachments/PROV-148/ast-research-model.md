# Codex lifecycle integration research

AST query `impl CompletionModel for CodexResponsesModel { $$$BODY }` in
`rust/providers/src/codex/model.rs` identifies the production implementation at line 21.
Its stream method currently delegates directly to shared ResponsesCompletionModel.
Replace this delegation with adapter-owned lifecycle processing; keep direct completion
delegation and non-Codex models unchanged. Public Responses request TryFrom and
GenericEventSource provide the needed wire and transport seams without shared edits.
