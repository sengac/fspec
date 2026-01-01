@caching
@tools
@high
@CACHE-001
Feature: Tool Result and File Content Cache
  """
  Implement a two-tier caching system inspired by vtcode to prevent redundant tool executions
  and reduce context window bloat from repeated file reads.

  Problem: Currently, when the model reads the same file multiple times (e.g., AgentModal.tsx
  was read 4 times in one turn), each read adds ~35k tokens to the context. This caused a
  simple prompt to consume 334k tokens.

  Solution: Implement file content cache + tool result cache with fuzzy matching, similar to
  vtcode's implementation which achieves 80-90% cache hit rates.

  Reference: /tmp/vtcode/vtcode-core/src/tools/cache.rs, smart_cache.rs, result_cache.rs
  """

  # ========================================
  # ARCHITECTURE OVERVIEW
  # ========================================
  #
  # Two-tier caching:
  #
  # 1. FILE CONTENT CACHE (Tier 1)
  #    - Caches raw file content by path
  #    - TTL: 5 minutes (configurable)
  #    - Memory limit: 50MB (configurable)
  #    - LRU eviction when capacity exceeded
  #    - Invalidated on file modification (via watcher or mtime check)
  #
  # 2. TOOL RESULT CACHE (Tier 2)
  #    - Caches tool execution results (grep, read, glob, etc.)
  #    - Content-addressed keys: tool_name:params_hash:target_path
  #    - Fuzzy matching at 0.85 threshold for similar queries
  #    - Selective path invalidation (not full cache clear)
  #    - Arc<String> storage for zero-copy retrieval
  #
  # DEDUPLICATION IN CONTEXT:
  #    - When returning cached results, append "(cached)" marker
  #    - For file reads: "See previous read of {path}" if exact match
  #    - For partial reads: Include only the NEW content not previously seen
  #
  # ========================================

  Background: User Story
    As a developer using codelet for code exploration
    I want repeated file reads and tool calls to be cached
    So that my context window isn't bloated with duplicate content

  # ----------------------------------------
  # FILE CONTENT CACHE SCENARIOS
  # ----------------------------------------

  Scenario: First read of a file populates the cache
    Given the file content cache is empty
    And a file "/project/src/App.tsx" exists with 1000 lines
    When the read tool is called for "/project/src/App.tsx"
    Then the file content should be returned
    And the file should be added to the content cache
    And the cache should store the file's mtime for invalidation

  Scenario: Second read of same file returns cached content
    Given a file "/project/src/App.tsx" has been read and cached
    When the read tool is called again for "/project/src/App.tsx"
    Then the cached content should be returned
    And the tool result should include "(cached)" marker
    And no filesystem read should occur

  Scenario: Partial read after full read returns reference
    Given a file "/project/src/App.tsx" has been fully read (lines 1-2000)
    When the read tool is called for "/project/src/App.tsx" with offset=500, limit=100
    Then the result should say "Lines 500-600 were included in the previous read of /project/src/App.tsx above"
    And no duplicate content should be added to context

  Scenario: Partial read of uncached region returns content
    Given a file "/project/src/App.tsx" has been read for lines 1-500
    When the read tool is called for "/project/src/App.tsx" with offset=1000, limit=100
    Then the new content (lines 1000-1100) should be returned
    And the cache should be updated to track the new range

  Scenario: File modification invalidates cache
    Given a file "/project/src/App.tsx" is cached
    When the file is modified (mtime changes)
    And the read tool is called for "/project/src/App.tsx"
    Then the cache entry should be invalidated
    And the fresh content should be read from disk
    And the cache should be updated with new content and mtime

  Scenario: Cache respects memory limit
    Given the file content cache has a 50MB limit
    And 45MB of files are already cached
    When a 10MB file is read
    Then LRU eviction should remove oldest entries
    And the new file should be cached
    And total cache size should remain under 50MB

  Scenario: Cache respects TTL
    Given a file was cached 6 minutes ago
    And the TTL is 5 minutes
    When the read tool is called for that file
    Then the stale cache entry should be evicted
    And the file should be re-read from disk

  # ----------------------------------------
  # TOOL RESULT CACHE SCENARIOS
  # ----------------------------------------

  Scenario: Grep results are cached
    Given the tool result cache is empty
    When grep is called with pattern "useState" in "/project/src"
    Then the grep results should be returned
    And the result should be cached with key "grep:hash(pattern,path):src"

  Scenario: Identical grep query returns cached result
    Given grep for "useState" in "/project/src" was previously executed
    When grep is called again with pattern "useState" in "/project/src"
    Then the cached result should be returned with "(cached)" marker
    And no grep execution should occur

  Scenario: Fuzzy matching catches similar grep queries
    Given grep for "useState" in "/project/src/components" was executed
    When grep is called for "useState" in "/project/src/components/"
    Then fuzzy matching should detect similarity >= 0.85
    And the cached result should be returned
    And the result should note it's from a similar query

  Scenario: File modification selectively invalidates tool cache
    Given grep results for "/project/src/App.tsx" are cached
    And grep results for "/project/src/Button.tsx" are cached
    When "/project/src/App.tsx" is modified
    Then only cache entries involving "App.tsx" should be invalidated
    And cache entries for "Button.tsx" should remain valid

  Scenario: Glob results are cached
    Given glob for "**/*.tsx" in "/project/src" was executed
    When glob is called again for "**/*.tsx" in "/project/src"
    Then the cached result should be returned
    And the result should include "(cached)" marker

  Scenario: Different tool parameters create separate cache entries
    Given read for "/project/src/App.tsx" with no offset is cached
    When read is called for "/project/src/App.tsx" with offset=1000
    Then this should be treated as a different cache key
    And a new cache entry should be created

  # ----------------------------------------
  # CONTEXT DEDUPLICATION SCENARIOS
  # ----------------------------------------

  Scenario: Multiple reads of same file in one turn are deduplicated
    Given a new conversation turn starts
    When the model reads "/project/src/App.tsx" at line 1
    And the model reads "/project/src/App.tsx" at line 500
    And the model reads "/project/src/App.tsx" at line 1000
    Then the first read should return full content
    And subsequent reads should return "See previous read above" references
    And total context added should be ~35k tokens, not ~105k tokens

  Scenario: Cache statistics are tracked
    Given the caching system is active
    When multiple tool calls are made during a session
    Then cache hit rate should be tracked
    And cache miss rate should be tracked
    And bytes saved should be calculated
    And statistics should be available via debug/status command

  # ----------------------------------------
  # CONFIGURATION SCENARIOS
  # ----------------------------------------

  Scenario: Cache can be disabled via configuration
    Given cache_enabled is set to false in config
    When the read tool is called
    Then no caching should occur
    And the file should always be read from disk

  Scenario: Cache TTL is configurable
    Given cache_ttl is set to 10 minutes in config
    When a file is cached
    Then the entry should expire after 10 minutes

  Scenario: Cache memory limit is configurable
    Given cache_memory_limit is set to 100MB in config
    When files are cached
    Then eviction should occur when total size exceeds 100MB

  Scenario: Fuzzy match threshold is configurable
    Given fuzzy_match_threshold is set to 0.90 in config
    When a similar query is made
    Then fuzzy matching should use 0.90 threshold instead of default 0.85

  # ----------------------------------------
  # IMPLEMENTATION NOTES
  # ----------------------------------------
  #
  # Location: codelet/tools/src/cache/
  #   - mod.rs: Cache module exports
  #   - file_cache.rs: FileContentCache implementation
  #   - result_cache.rs: ToolResultCache implementation
  #   - fuzzy.rs: Fuzzy matching utilities (Levenshtein or similar)
  #
  # Integration points:
  #   - codelet/tools/src/read.rs: Use FileContentCache
  #   - codelet/tools/src/grep.rs: Use ToolResultCache
  #   - codelet/tools/src/glob.rs: Use ToolResultCache
  #   - codelet/cli/src/interactive/stream_loop.rs: Clear turn-local dedup on new turn
  #
  # Data structures (inspired by vtcode):
  #   - FileContentCache: HashMap<PathBuf, CachedFile> with LRU eviction
  #   - CachedFile: { content: Arc<String>, mtime: SystemTime, size: usize, last_access: Instant }
  #   - ToolResultCache: HashMap<String, CachedResult> with fuzzy index
  #   - CachedResult: { result: Arc<String>, created: Instant, access_count: u32 }
  #
  # Thread safety:
  #   - Use Arc<RwLock<Cache>> for concurrent access
  #   - Tools are called concurrently, cache must be thread-safe
  #
  # ========================================
