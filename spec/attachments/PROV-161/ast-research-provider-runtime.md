# AST research: provider runtime integration

Executed AstGrep against the Rust session and provider source trees before testing.

## Session image settings

Pattern: `pub fn resolve_profile_max_images($$$ARGS) -> $RET { $$$BODY }`

Match: `rust/sessions/src/model_resolution.rs:83`,
`pub fn resolve_profile_max_images(pm: &ProviderManager) -> Option<u32>`.

This existing integration point reads only selected profiles. Provider defaults
must enter the same lifecycle path rather than a disconnected configuration helper.

## Provider output ceilings

Pattern: `fn max_output_tokens_limit(&self) -> Option<usize> { $$$BODY }`

Matches include:

- `rust/providers/src/claude.rs:187`
- `rust/providers/src/manager.rs:185`
- `rust/providers/src/codex/mod.rs:122`
- `rust/providers/src/openai.rs:104`
- `rust/providers/src/gemini.rs:50`
- `rust/providers/src/zai.rs:71`
- `rust/providers/src/copilot/provider.rs:213`
- Trait declaration and test implementation in `rust/providers/src/model_limits.rs`.

Both the concrete Claude resolver and the lightweight manager resolver participate
in limits. Changing only one would leave selection and physical requests inconsistent.
Codex wire omission must remain separate from the effective admission reservation.

Detailed read-only investigation and code references are in the attached
`provider-settings-parity-research.md`.
