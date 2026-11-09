@BUG-582
@done
@model-selection
@rust
@thinking-level
@providers
@high
@MODEL-014
Feature: Codex thinking levels come from the Codex /models endpoint
  """
  Codex thinking levels come from the Codex backend GET /models, fetched with ChatGPT sign-in and a pinned
  client_version, instead of models.dev's OpenAI reasoning_options (MODEL-009 research §1.6, §3, §8.2).
  - codelet_providers::codex::models_catalog: CODEX_MODELS_CLIENT_VERSION (0.165.0), the fetch
  (GET <base>/models?client_version=..., RefreshingCodexClient in OAuth mode: Bearer, ChatGPT-Account-Id,
  token refresh) and CodexCatalog, which resolves an entry into a MODEL-009 ReasoningCapability and a
  PROV-152 CodexCapabilities snapshot (efforts, default effort). ultra and persistent are dropped first.
  - ProviderManager holds the catalog result for the selection in memory (no disk cache). sessions
  model_resolution::resolve_provider_manager_and_limits fetches it for codex models on session create,
  resume and set_model; turns do not fetch. ThinkingTarget::for_provider_manager reads it for the Codex
  provider; ProviderManager::get_codex sets the provider snapshot from it.
  - PROV-152: a CodexCapabilities control the snapshot does not list is not advertised and is sent
  unvalidated (BUG-441); a listed control keeps its checks.
  - The Codex section still lists the whole models.dev OpenAI catalog (PROV-129); PROV-176 narrows it.
  - Research: spec/attachments/MODEL-014/codex-models-endpoint-research.md and
  codex-models-response-capture.json (the /models responses below are cut from that capture).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A Codex model's thinking levels come only from the Codex backend GET /models response: the efforts in its entry's supported_reasoning_levels, matched by slug to the selected model id. models.dev reasoning_options and the bundled Codex catalog are not used for Codex models (product owner, 2026-11-04, decision 2; no fallbacks)
  #   2. A Codex model that the /models response does not list, or lists with no supported_reasoning_levels, has no reasoning control; the thinking dialog and /thinking say why, naming the model (standing decision: a model with no capability has no reasoning control)
  #   3. When the /models response cannot be had (no ChatGPT sign-in, network failure, an error status, or a body fspec cannot read), every Codex model has no reasoning control and the reason names the failure; nothing is substituted from models.dev, a bundled list or an earlier session (standing decisions: no silent fallbacks; a model with no capability has no reasoning control)
  #   4. Only ChatGPT sign-in (Codex OAuth tokens with an account id) can call /models: the request carries the Bearer access token and the ChatGPT-Account-Id header, refreshing an expired token first, as Codex responses requests do. Codex in API-key mode has no ChatGPT token, so its models have no reasoning control, with a reason saying the levels need ChatGPT sign-in (decision 2: ChatGPT auth + client_version)
  #   5. A level Codex does not advertise for the model is refused before any request is sent, naming the model and the levels Codex advertises; it is never remapped to a nearby level (standing decision; MODEL-009 rule)
  #   6. A saved level (MODEL-012) that the model's /models entry does not advertise leaves the session's level not set, and the user is told why (product owner, 2026-11-04, decision 7)
  #   7. An entry fspec cannot use fails only that model's capability, naming the model and the problem: an effort that is not a valid effort name (MODEL-009 ReasoningLevel::effort), or a default_reasoning_level that is not among its supported_reasoning_levels; the other models of the response keep theirs (MODEL-009 pattern for unreadable catalog entries)
  #   8. The Codex provider's request-option snapshot (CodexProvider::set_capabilities, PROV-152) is set in production for the selected model from the same /models entry: its advertised efforts and its default_reasoning_level as the advertised default effort; a model with no usable entry gets no snapshot (BUG-441: nothing invented)
  #   9. Request controls the /models entry does not advertise as a list of values (reasoning summary, reasoning context, text verbosity) are not advertised, so they are sent unvalidated as BUG-441 does without a snapshot, instead of being refused as unsupported; PROV-152's empty-list-means-unsupported reading applies only to a control the snapshot lists
  #   10. When the session's thinking level is not set (Model default), the Codex request carries the model's advertised default_reasoning_level as its effort, as Codex itself does (codex-rs build_reasoning); the PROV-152 advertised-default rule applies it, and an explicitly chosen level always wins
  #   11. fspec sends a Codex client version pinned in fspec and bumped with Codex releases, 0.165.0 (the version the live capture was fetched with), held in one named constant whose comment says where it comes from and that it is bumped with Codex releases (product owner, 2026-11-06, option b)
  #   12. fspec fetches /models once per model selection (session create, resume, model switch) and holds the result in memory for that session only, with no disk cache; a failed fetch means no reasoning control with the reason shown until the next selection retries (product owner, 2026-11-06, option a)
  #   13. ultra is not offered on Codex, since fspec has no delegation to back it, and Codex's persistent level is treated the same way if it is ever advertised: neither is part of a Codex model's capability or of the provider snapshot, so choosing one is refused like any level the model does not offer (product owner, 2026-11-06, option b)
  #   14. Confirmed by the product owner (2026-11-06): the derived rules stand — Codex in API-key mode has no reasoning control; with no level set the advertised default_reasoning_level is sent; a default outside the model's own level list loses that model's capability; and the PROV-152 change: request controls the /models entry does not list are not advertised (unvalidated) instead of unsupported
  #   15. The efforts a Codex model offers are its advertised efforts minus ultra and persistent, in MODEL-009's canonical order (none < minimal < low < medium < high < xhigh < max, other advertised names after them in Codex's order), all of them, nothing behind More... (product owner, 2026-11-04, decision 10; 2026-11-06 answer on ultra)
  #
  # EXAMPLES:
  #   1. Codex /models lists gpt-6-luna with supported_reasoning_levels low, medium, high, xhigh, max and default medium: the thinking dialog for codex/gpt-6-luna offers low, medium, high, xhigh, max in that order and Model default, where models.dev's openai/gpt-6-luna list (none, low, medium, high, xhigh, max) offered none as well
  #   2. On codex/gpt-6-luna, /thinking none is refused before anything is sent, naming codex/gpt-6-luna and listing low, medium, high, xhigh, max
  #   3. At xhigh on codex/gpt-6-luna the Codex request carries reasoning effort xhigh unchanged
  #   4. The /models response does not list gpt-5.5: codex/gpt-5.5 has no reasoning control, and the dialog and /thinking say Codex /models does not list gpt-5.5, where models.dev's list for openai/gpt-5.5 was offered before
  #   5. The /models request fails with 401 (or the network is down): every Codex model has no reasoning control and the reason names the failed /models request; no models.dev levels are offered instead
  #   6. Codex is signed in with an API key only: codex/gpt-6-luna has no reasoning control, with a reason saying Codex thinking levels need ChatGPT sign-in
  #   7. The /models request sent by fspec is GET https://chatgpt.com/backend-api/codex/models?client_version=<version> with Authorization Bearer <access token> and ChatGPT-Account-Id <account id>; an expired access token is refreshed first
  #   8. The /models response lists codex-auto-review with default_reasoning_level turbo, which is not among its levels low..max: codex/codex-auto-review has no reasoning control with a reason naming the model and turbo, and codex/gpt-6-luna in the same response keeps its levels
  #   9. The /models response lists a model with an effort named Extra High (not a valid effort name): only that model's capability fails, naming the model and the name
  #   10. xhigh is saved for codex/gpt-6-luna: a new session on codex/gpt-6-luna starts at xhigh. none is saved for codex/gpt-6-luna (from an older catalog): the session starts with the level not set and a notice says gpt-6-luna does not offer none
  #   11. A codex/gpt-6-sol session with the level not set sends reasoning effort medium, gpt-6-sol's advertised default; at low it sends low
  #   12. The Codex provider for codex/gpt-6-luna gets a snapshot advertising efforts low..max and default effort medium, so a request option asking for effort none is refused by the provider with the advertised efforts, and the MODEL-010 form's reasoning summary auto is still sent because /models lists no summary values
  #   13. Codex /models lists gpt-6.1-sol with low, medium, high, xhigh, max, ultra: the dialog offers low, medium, high, xhigh, max, and /thinking ultra on codex/gpt-6.1-sol is refused naming the model and listing low, medium, high, xhigh, max
  #   14. fspec's /models request carries client_version=0.165.0, the pinned Codex client version
  #   15. A session created on codex/gpt-6-luna fetches /models once; switching the session to codex/gpt-6-sol fetches it again; a turn does not fetch it. A fetch that failed at creation is retried by the next model switch, which then offers the levels
  #
  # QUESTIONS (ANSWERED):
  #   Q: Which client_version should fspec send to Codex GET /models? The backend uses it to decide which models to return: Codex's bundled catalog gives each model a minimal_client_version (gpt-6-astra 0.153.0, gpt-6-sol 0.155.0, gpt-5.5 0.124.0), and the Codex CLI sends its own version (0.165.0 in the live capture). fspec's own version is 0.10.9, which would probably get few or no current models back. Options: (a) fspec's own version 0.10.9; (b) a Codex client version pinned in fspec and updated with releases; (c) a value the user sets in fspec-config (e.g. providers.codex.clientVersion), with no reasoning control until it is set; (d) something else.
  #   A: fspec sends a Codex client version pinned in fspec and bumped with Codex releases, 0.165.0 (the version the live capture was fetched with), held in one named constant whose comment says where it comes from and that it is bumped with Codex releases (product owner, 2026-11-06, option b)
  #
  #   Q: When does fspec fetch Codex /models, and how stale may the result get? Options: (a) once per model selection (session create, resume, model switch), held in memory for that session only, with no disk cache, so a long session keeps what it fetched and a failed fetch is retried at the next selection; (b) like the Codex CLI: a disk cache (~/.fspec/cache) with a TTL (Codex uses 5 minutes plus an etag check) that is refreshed before turns; (c) another policy. And with (b), may a cached response older than the TTL still be used when /models is unreachable, or does an expired cache mean no reasoning control?
  #   A: fspec fetches /models once per model selection (session create, resume, model switch) and holds the result in memory for that session only, with no disk cache; a failed fetch means no reasoning control with the reason shown until the next selection retries (product owner, 2026-11-06, option a)
  #
  #   Q: Codex advertises ultra ("Maximum reasoning with automatic task delegation") on gpt-6-astra, gpt-6-sol, gpt-6.1-sol, gpt-6.1-terra, gpt-5.6-sol and gpt-5.6-terra. The Codex CLI never sends ultra over the wire. It resolves ultra to the model's multi_agent_reasoning_effort (xhigh) or max, and it also turns on its own multi-agent delegation. fspec has no such delegation. Should fspec (a) offer ultra and send effort ultra unchanged (the MODEL-010 rule; the server may reject it), (b) not offer ultra on Codex, or (c) offer it and send what Codex sends (multi_agent_reasoning_effort, else max)? Codex's persistent level (sent as disabled) raises the same question if it is ever advertised.
  #   A: ultra is not offered on Codex, since fspec has no delegation to back it, and Codex's persistent level is treated the same way if it is ever advertised: neither is part of a Codex model's capability or of the provider snapshot, so choosing one is refused like any level the model does not offer (product owner, 2026-11-06, option b)
  #
  # ========================================
  Background: User Story
    As a user of the Codex provider signed in with ChatGPT
    I want to be offered exactly the thinking levels Codex advertises for my model, taken from the Codex /models endpoint
    So that I can choose every level Codex supports (xhigh, max) and never one Codex does not list, with no catalog guessing

  Scenario: The thinking dialog offers the levels Codex /models advertises
    Given Codex /models lists gpt-6-luna with levels low, medium, high, xhigh, max and default medium
    When a session starts on codex/gpt-6-luna
    Then the thinking options are low, medium, high, xhigh, max in that order
    And none is not offered

  Scenario: A level Codex does not advertise is refused and an advertised one is sent unchanged
    Given a session on codex/gpt-6-luna whose /models entry lists low, medium, high, xhigh, max
    When the user chooses none
    Then it is refused naming codex/gpt-6-luna and listing low, medium, high, xhigh, max
    When the user chooses xhigh
    Then the Codex request carries reasoning effort xhigh

  Scenario: Ultra is not offered on Codex
    Given Codex /models lists gpt-6.1-sol with levels low, medium, high, xhigh, max, ultra
    When a session starts on codex/gpt-6.1-sol
    Then the thinking options are low, medium, high, xhigh, max in that order
    And choosing ultra is refused naming codex/gpt-6.1-sol and listing low, medium, high, xhigh, max

  Scenario: A Codex model that /models does not list has no reasoning control
    Given the Codex /models response does not list gpt-5.5
    When a session starts on codex/gpt-5.5
    Then it has no reasoning control because Codex /models does not list gpt-5.5

  Scenario: A failed /models request leaves no reasoning control until the next selection retries
    Given the Codex /models request fails with status 401
    When a session starts on codex/gpt-6-luna
    Then it has no reasoning control and the reason names the failed Codex /models request
    When /models answers again and the session switches to codex/gpt-6-luna
    Then the thinking options are low, medium, high, xhigh, max in that order

  Scenario: Codex signed in with an API key only has no reasoning control
    Given Codex is signed in with an API key and no ChatGPT tokens
    When a session starts on codex/gpt-6-luna
    Then it has no reasoning control because Codex thinking levels need ChatGPT sign-in
    And no /models request is sent

  Scenario: The /models request carries the pinned client version and ChatGPT auth
    Given Codex is signed in with ChatGPT as account acc_model014
    When a session starts on codex/gpt-6-luna
    Then one GET /models request is sent with client_version 0.165.0, the pinned Codex client version
    And it carries the Bearer access token and ChatGPT-Account-Id acc_model014
    When the access token has expired
    Then the token is refreshed before the /models request is sent

  Scenario: An entry fspec cannot use fails only that model's capability
    Given a /models response listing codex-auto-review with default turbo outside its levels, gpt-6-sol with an effort named Extra High, and gpt-6-luna
    When the Codex capabilities are resolved
    Then codex/codex-auto-review has no reasoning control with a reason naming it and turbo
    And codex/gpt-6-sol has no reasoning control with a reason naming it and Extra High
    And codex/gpt-6-luna offers low, medium, high, xhigh, max

  Scenario: A saved level applies only when Codex advertises it
    Given the saved thinking level for codex/gpt-6-luna is xhigh
    When a session starts on codex/gpt-6-luna
    Then its thinking level is xhigh
    Given the saved thinking level for codex/gpt-6-luna is none
    When a session starts on codex/gpt-6-luna
    Then its thinking level is not set and a warning says codex/gpt-6-luna does not offer none

  Scenario: With no level set the request carries the advertised default effort
    Given the Codex provider is built for codex/gpt-6-sol, whose /models default is medium
    When a request is prepared with no thinking level
    Then it carries reasoning effort medium
    When a request is prepared at low
    Then it carries reasoning effort low

  Scenario: The provider snapshot comes from /models and leaves unlisted controls unvalidated
    Given the Codex provider is built for codex/gpt-6-luna from its /models entry
    When a request option asks for reasoning effort none
    Then the provider refuses reasoning.effort none and lists low, medium, high, xhigh, max
    When a request asks for reasoning effort xhigh with summary auto
    Then the request carries effort xhigh and summary auto

  Scenario: /models is fetched once per model selection and not per turn
    Given a session on codex/gpt-6-luna has fetched Codex /models once
    When the session prepares a turn
    Then no further /models request is sent
    When the session switches to codex/gpt-6-sol
    Then a second /models request is sent and gpt-6-sol's levels are offered
