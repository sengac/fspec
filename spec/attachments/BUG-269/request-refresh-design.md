# BUG-269 bounded request-time integration design

Source review confirms all four real custom completion front doors already call
`RhaiCustomProvider::invoke_build_headers`. Use that single integration point,
before reading script headers, rather than introducing another HTTP dialect or
inference retry implementation. Bind the exact selected compiled engine, AST and
ProviderConfig; never rediscover a global script by slug, because project-local
resolution and provider-scoped modules must remain authoritative.

Automatic middleware applies to explicit OAuth device/PKCE configurations with
the supported predicate/refresh hook pair, including legacy aliases. The
existing `AuthConfig::Custom` contract explicitly owns authentication in the
script and remains untouched, even if auxiliary OAuth hooks exist. Bearer/API
key configurations also remain untouched. Missing hooks leave the existing
headers-owned path intact rather than inventing a new login policy. This retains
self-refreshing scripts without duplicate refresh. Existing native providers
never call this custom integration.

Reuse ScriptedRefreshingClient's existing stored-token/predicate/refresh/persist
sequence. Introduce a small process-local weak lock table keyed by the resolved
credential-store path, containing locks only, not tokens or session state. Lock
before reading/rechecking tokens so independently reconstructed providers sharing
one credential store coalesce normally. No inference retries, new issuer policy,
new refresh retry/deadline, persistent configuration or installed-script audit.

The script's synchronous HTTP runs on existing spawn_blocking paths. A caller
cancelled or timed out during refresh must never proceed to inference. To retain
single-flight ownership until a synchronous callback really finishes, the
bounded refresh operation owns its lock independently of the inference caller;
its authorized refresh may finish and persist normally even if that caller has
gone away. Capture the credential-store path at admission and fail closed if the
active root changes before persistence. Tests explicitly wait for admitted
refresh settlement before restoring their temporary environment. No uncancelled
request or orphan task may access restored user credentials after fixture exit.

Returned request diagnostics identify only the provider and refresh stage, not
script exception text, token strings or issuer bodies. Preserve the helper's
public error behavior for its existing callers; the new request integration
redacts at its boundary. Rejection/transport failure sends zero inference, leaves
tokens unchanged, and allows a later explicit request to retry by ordinary
predicate evaluation. This adds no automatic retry or error reclassification to
the shared/native providers.

Required actual-request RED/GREEN coverage: all streaming/nonstreaming front
doors with preferred/legacy hooks and actual local issuer Authorization;
independent concurrent providers coalescing; fresh/missing credentials and
headers-owned script controls; project-local script precedence; rejected issuer,
transient transport failure, cancellation and external caller deadline, with
zero forbidden inference and safe diagnostics. All fixtures are temporary,
synthetic, mock-keyring and numeric-loopback-only.
