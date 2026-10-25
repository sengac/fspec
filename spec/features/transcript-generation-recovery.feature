@done
@tui
@rpc
@persistence
@agent-view
@session-management
@BUG-256
Feature: Recover TUI transcript hydration after a server generation change
  """
  Keep generation mismatch typed from BackendError through TranscriptAction, distinct from transient transport errors. On known mismatch replace per-tab hydration and rendered-generation state together, rotating nonce and dropping abort handles. Request a fresh unpinned snapshot; reject old nonce/request responses. Ordinary resume/reconnect retains dedup unless invalidated. Mechanically move resume/failure handling to a focused sibling module to keep dispatch below 300 lines. Integration tests use the real SessionManager and embedded RPC backend with temporary stores, no external provider or listening socket.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Known generation mismatch atomically invalidates cursor, partial bytes, pinned snapshot and inflight identity; /resume can hydrate the new generation without reopening. Replace old generation rendering, never append a second full snapshot.
  #   2. Transport failures remain distinguishable and retain displayed content and deduplication. Late responses from an invalidated hydration attempt must not mutate current rendering, including an archive-install race.
  #
  # EXAMPLES:
  #   1. An open tab misses clear, then /resume displays new-generation content once in order and no old-generation text, without closing the tab.
  #   2. Archive installation changes generation after a partial page; recovery discards the old bytes and late page, then renders the authoritative transcript once. A normal reconnect retains existing content once.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should reconnect reset deduplication for all failures or only when generation invalidation is established?
  #   A: Only known generation invalidation resets hydration. As required by the attachment, transient transport failures and unchanged-generation reconnect retain cursor/dedup and displayed content. Typed generation mismatch must survive backend conversion. A reset replaces old display with authoritative new-generation content and invalidates partial/snapshot/inflight responses atomically.
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to resume an open tab after its server transcript generation changes
    So that new content appears once without reopening or mixing old partial data

  Scenario: Resume recovers a missed server clear without reopening
    Given an open tab displaying content from a previous server generation
    When the server clears without delivering its lifecycle event and the user resumes
    Then only new generation content renders exactly once in order

  Scenario: Archive installation during partial hydration recovers atomically
    Given a tab with a partial payload and pinned snapshot from an unseeded generation
    When archive installation changes generation and an old page arrives late
    Then old bytes and late responses are discarded and authoritative history renders once

  Scenario: Unchanged generation reconnect preserves deduplication
    Given an open hydrated tab with existing transcript content
    When the user resumes twice without a generation change
    Then existing and subsequent content each render exactly once

  Scenario: Transient page failures preserve displayed history
    Given an open hydrated tab with existing transcript content
    When a transport failure occurs before resume succeeds
    Then displayed content remains intact and catch up renders each event once
