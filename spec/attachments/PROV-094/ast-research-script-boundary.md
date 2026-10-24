# Scripted Codex integration research

AstGrep found `RhaiCustomProvider::invoke_build_headers` in
`rust/providers/src/custom/provider_dispatch.rs:84`. The seven script lifecycle
functions are validated by ScriptLoader. The common request bridge currently
omits model and system-prefix context, so a bundled script cannot build parity
requests without an additive bridge change. The default sandbox crypto module
has base64url_encode but no decode. Provider-scoped cred functions already
provide isolated file storage. ScriptedOAuthProvider receives auth_url,
token_url and client_id, enabling a loopback test issuer without live credentials.

The bundle remains optional: users copy templates into their providers directory.
No native provider routing, rig patch, automatic shadowing or keychain changes
are required. The native Codex facade is available through tool_style=codex;
the script's wire builder preserves names supplied by that facade.
