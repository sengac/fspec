# PROV-094 / BUG-339 research (2026-11-04)

Sources: the built-in CodexProvider (rust/providers/src/codex/), the
scripted-provider engine (rust/providers/src/custom/, oauth/), the test
rust/providers/tests/prov094_codex_bundle.rs, and upstream openai/codex
(codex-rs, cloned locally for reading only).

## What the bundle test expects
- Bundle dir `rust/providers/templates/codex/` (codex.json + codex.rhai); never
  existed in git history.
- codex.json: `models.default`, `system_prompt.prefix` == CODEX_BASE_INSTRUCTIONS
  (rust/providers/src/codex/codex_base_instructions.md, exact bytes),
  `tool_style: "codex"`.
- Script must define build_request, build_headers, build_url, parse_response,
  parse_stream_chunk(config, data), build_stream_request, map_error(status, body)
  (custom/script_loader.rs:22), plus OAuth hooks auth_start, poll_for_token
  (legacy name), auth_needs_refresh (expires_at == 0), auth_refresh.
- OAuth (default engine — no cred::):
  - auth_start: form-encoded POST `{auth_url}/api/accounts/deviceauth/usercode`
    with client_id=app_EMoamEEZ73f0CkXaXp7hrann; returns user_code.
  - poll: 200 {"error":"authorization_pending"} => pending; 200
    {authorization_code, code_verifier} => form POST token_url
    (grant_type=authorization_code, code, code_verifier, client_id,
    redirect_uri={issuer}/deviceauth/callback) => success + tokens + account_id.
  - refresh: form grant_type=refresh_token&refresh_token=..., content-type
    application/x-www-form-urlencoded; account_id from the id_token JWT
    top-level chatgpt_account_id.
- Request parity:
  - URL https://chatgpt.com/backend-api/codex/responses.
  - Headers: authorization Bearer, ChatGPT-Account-Id, originator: rust.
  - Body: model, instructions (system message), stream:true, store:false,
    reasoning (merged from thinking_config), input (input_text, function_call,
    function_call_output) and tools [{type:function, name, description,
    parameters}].
- Stream:
  - output_text.delta => text_delta.
  - reasoning_summary_text.delta => reasoning_delta.
  - output_item.done function_call => tool_call (id = call_id).
  - response.completed => usage + stop.
  - response.failed => throw.

## Engine gaps to close
1. crypto::base64url_decode missing (oauth/building_blocks.rs build_crypto_module).
2. cred::read("codex") reads $FSPEC_HOME/codex.json but write_stored_tokens writes
   $FSPEC_HOME/oauth/codex.json (cred_module.rs:162 vs custom_oauth.rs:95).
3. Request map lacks model and system prefix (request_bridge.rs:66) — add them.
4. Test asserts tools parameters == raw input_schema, but build_stream_request
   decorates every schema with a required work_unit_id (tool_contract.rs) — the
   test must expect the decorated schema.
5. No clock in the sandbox (expires_at only via == 0).
6. http::post throws on 4xx/5xx, so 403/404 "pending" cannot be handled in script.
7. OAuth hooks run in the default engine: no cred:: there.

## Upstream differences (reference only)
Upstream uses JSON for usercode and refresh (+refresh_reason), originator
codex_cli_rs, nested auth claim for account id, base instructions as an input
fragment, tool_choice/parallel_tool_calls/include/client_metadata. The bundle
targets parity with OUR built-in provider, not upstream.
