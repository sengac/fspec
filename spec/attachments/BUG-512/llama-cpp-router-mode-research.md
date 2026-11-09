# BUG-512: how llama.cpp router mode exposes a model's template capabilities

Source: ggml-org/llama.cpp, shallow clone of master at 6c01da6278ab6c69ed67d9124806b50d2b299ba7 (2026-11-06), in /tmp/llama.cpp.

## Single-model server
- `GET /props` handler ignores the request entirely (`tools/server/server-context.cpp:5894`, `[this](const server_http_req &)`),
  and returns `chat_template_caps` (`server-context.cpp:5713`), built from the loaded template
  (`server-context.cpp:5271`). Query parameters such as `?model=` are ignored.

## Router mode (several models behind one server)
- The router registers its own `/props` handler (`tools/server/server.cpp:526`, `routes.get_props = models_routes->get_router_props`).
- `get_router_props` (`tools/server/server-models.cpp:2021-2046`): with no `model` query parameter it answers the router's own
  props: `role: "router"`, a dummy `model_alias: "llama-server"`, `default_generation_settings {params: {}, n_ctx: 0}`, and
  NO `chat_template_caps`. With `?model=<name>` it proxies the request to that model's child server (`proxy_get`).
- `proxy_get` (`server-models.cpp:2049-2064`): validates the model with `router_validate_model`, and, when autoload is on,
  loads the model first (`ensure_model_ready`, blocks until the child is ready) before proxying.
- `router_validate_model` (`server-models.cpp:1949-1966`): empty name -> 400 "model name is missing from the request";
  unknown name -> 400 "model '<name>' not found"; aliases resolve to the canonical name; with autoload off and the model
  not running -> 400 "model is not loaded". Errors use `format_error_response` (`{"error": {"code", "message", "type"}}`).
- `is_autoload` (`server-models.cpp:1968-1975`): the per-request `?autoload=true|false` query parameter overrides the server's
  `--models-autoload` default.
- README (`tools/server/README.md:2157-2165`): "For GET endpoints (/props, /metrics, etc.) The router uses the model query
  parameter (URL-encoded): GET /props?model=ggml-org%2Fgemma-3-4b-it-GGUF%3AQ4_K_M"; "you can include ?autoload=true|false
  in the query param to control this behavior per-request".
- `GET /v1/models` in router mode (`server-models.cpp:2133-2225`) lists id, aliases, status, architecture, memory and the
  child's `loaded_info` (architecture, download progress); it carries no `chat_template_caps`, so it can't serve the check.

## Consequence for fspec's MODEL-016 live check
- Today fspec sends plain `GET <root>/props`. On a router this returns the router's own props, which have no
  `chat_template_caps.reasoning_effort_levels`, so the check fails with a misleading reason and never checks the model.
- Fix: send `GET <root>/props?model=<model id>&autoload=false`. A single-model server ignores both parameters (unchanged
  behaviour). A router proxies to the selected model's own server, whose `chat_template_caps` describe that model.
- `autoload=false`: the live check only reports (MODEL-016 rule 14), so it must not make the router load a model, which can
  evict other loaded models (LRU, `--models-max`) and blocks past the 5 s check timeout. A router that has not loaded the
  model answers 400 "model is not loaded"; that is reported as a check that could not be made, with the server's reason,
  and the declaration still applies (MODEL-016 rule 15).

## Same pattern elsewhere (not this card)
- `rust/providers/src/engine_limits.rs` (BUG-547) reads llama.cpp `GET /props default_generation_settings.n_ctx` without
  `?model=`; on a router that is the dummy `n_ctx: 0`, reported as not a token count.
