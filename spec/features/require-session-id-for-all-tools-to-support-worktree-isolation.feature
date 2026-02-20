@TOOL-014
Feature: Require session_id for all tools to support worktree isolation

  """
  ============================================================================
  MANDATORY REQUIREMENTS - NON-NEGOTIABLE
  ============================================================================

  REQUIREMENT 1: ALL TOOL CONSTRUCTORS MUST REQUIRE session_id PARAMETER
    - ReadTool::new(session_id: Uuid) - REQUIRED
    - WriteTool::new(session_id: Uuid) - REQUIRED
    - EditTool::new(session_id: Uuid) - REQUIRED
    - GrepTool::new(session_id: Uuid) - REQUIRED
    - GlobTool::new(session_id: Uuid) - REQUIRED
    - LsTool::new(session_id: Uuid) - REQUIRED
    - AstGrepTool::new(session_id: Uuid) - REQUIRED
    - AstGrepRefactorTool::new(session_id: Uuid) - REQUIRED
    - WebSearchTool::new(session_id: Uuid) - REQUIRED
    - SearchToolFacadeWrapper::new(session_id: Uuid) - REQUIRED
    - LsToolFacadeWrapper::new(session_id: Uuid) - REQUIRED

  REQUIREMENT 2: NO DEFAULT TRAIT IMPLEMENTATIONS
    - Tools MUST NOT implement Default trait
    - There is no valid default without a session_id
    - Compilation MUST fail if tool is constructed without session_id

  REQUIREMENT 3: ALL PROVIDERS MUST PASS session_id TO ALL TOOLS
    - Claude provider MUST pass session_id to all tools
    - OpenAI provider MUST pass session_id to all tools
    - Codex provider MUST pass session_id to all tools
    - Gemini provider MUST pass session_id to all tools
    - ZAI provider MUST pass session_id to all tools

  REQUIREMENT 4: PATH VALIDATION IN ISOLATED SESSIONS
    - Tools MUST call get_effective_cwd(session_id) to resolve file paths
    - Tools MUST reject absolute paths outside worktree when session is isolated
    - Tools MUST resolve relative paths to worktree directory

  ============================================================================
  IMPLEMENTATION NOTES
  ============================================================================

  Pattern: Store session_id field, implement get_effective_cwd() method, resolve paths before file operations
  FspecTool and BridgeTool already have session_id - verify they use it for path resolution
  Helper: validate_and_resolve_path(session_id: Uuid, path: &str) -> Result<PathBuf, ToolError>
  Shared lookup: 1) Call get_effective_cwd(session_id) 2) If Some(worktree_path), resolve relative paths to worktree and reject absolute paths outside worktree 3) If None, operate normally
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All tool constructors MUST take session_id: uuid::Uuid as REQUIRED parameter - NO EXCEPTIONS
  #   2. Tools MUST call get_effective_cwd(session_id) to resolve file paths in isolated sessions
  #   3. Tools MUST NOT implement Default trait (cannot default without session_id)
  #   4. All providers MUST pass session_id when constructing tools - NO EXCEPTIONS
  #   5. Tools MUST validate file paths - if worktree exists, reject operations outside worktree
  #   6. Tools MUST return error if attempting to read/write outside worktree when session is isolated
  #   7. All tools MUST use shared get_effective_cwd(session_id) from codelet_tools::facade
  #
  # EXAMPLES:
  #   1. ReadTool::new(session_id) reads file from worktree path in isolated session
  #   2. WriteTool::new(session_id) writes file to worktree path in isolated session
  #   3. GrepTool::new(session_id) searches within worktree directory in isolated session
  #   4. Tests use Uuid::nil() for session_id since they don't need worktree isolation
  #   5. WriteTool in isolated session attempts to write to /project/src/file.rs but worktree is at /project/.fspec/worktrees/abc123/ → error: path outside worktree
  #   6. ReadTool in isolated session reads 'src/file.rs' (relative) → resolves to /project/.fspec/worktrees/abc123/src/file.rs
  #   7. EditTool in isolated session given absolute path /project/src/file.rs → error because path is in main directory, not worktree
  #   8. GlobTool in isolated session searches '**/*.rs' → only finds files within worktree, not main project
  #   9. AstGrepTool in isolated session with path='src/' → searches /project/.fspec/worktrees/abc123/src/, not /project/src/
  #   10. GrepTool in isolated session with path='/project/src' (absolute, main dir) → error: path outside worktree
  #   11. LsTool in isolated session lists '.' → lists worktree root, not main project root
  #   12. AstGrepRefactorTool in isolated session refactors 'src/lib.rs' → modifies worktree copy, main project unchanged
  #   13. ReadTool detects path /project/src/main.rs is outside worktree /project/.fspec/worktrees/abc123/ → returns ToolError::Validation { tool: 'read', message: 'Path is outside isolated worktree. Use relative path or path within worktree.' }
  #   14. WriteTool detects path /project/src/new.rs is outside worktree → returns ToolError::Validation { tool: 'write', message: 'Cannot write outside isolated worktree. Session worktree: .fspec/worktrees/abc123/' }
  #   15. EditTool detects old_string replacement targets /project/src/lib.rs outside worktree → returns ToolError::Validation { tool: 'edit', message: 'Cannot edit file outside isolated worktree' }
  #   16. GrepTool detects search path /project/src outside worktree → returns ToolError::Validation { tool: 'grep', message: 'Search path is outside isolated worktree' }
  #   17. GlobTool detects path parameter /project/src outside worktree → returns ToolError::Validation { tool: 'glob', message: 'Glob base path is outside isolated worktree' }
  #   18. LsTool detects directory /project/src outside worktree → returns ToolError::Validation { tool: 'ls', message: 'Cannot list directory outside isolated worktree' }
  #   19. AstGrepTool detects search path /project/src outside worktree → returns ToolError::Validation { tool: 'ast_grep', message: 'AST search path is outside isolated worktree' }
  #   20. AstGrepRefactorTool detects source_file /project/src/lib.rs outside worktree → returns ToolError::Validation { tool: 'ast_grep_refactor', message: 'Cannot refactor file outside isolated worktree' }
  #   21. BashTool in isolated session runs 'pwd' → outputs worktree path /project/.fspec/worktrees/abc123/, not /project/
  #   22. When session uses Uuid::nil() (tests/non-isolated), get_effective_cwd returns None → tools operate in current directory without path validation
  #
  # ========================================

  Background: User Story
    As a developer
    I want to have all tools use session_id for worktree lookup
    So that file operations work correctly in isolated sessions

  # ============================================================================
  # REQUIREMENT 1: CONSTRUCTOR SIGNATURE - session_id IS MANDATORY
  # ============================================================================
  # These scenarios verify that ALL tools REQUIRE session_id in their constructor.
  # This is a compile-time guarantee - tools cannot be instantiated without session_id.
  # ============================================================================

  Scenario: ReadTool REQUIRES session_id parameter in constructor
    Given ReadTool is being instantiated
    Then the constructor signature MUST be ReadTool::new(session_id: Uuid)
    And calling ReadTool::new() without session_id MUST fail to compile
    And ReadTool MUST NOT implement Default trait

  Scenario: WriteTool REQUIRES session_id parameter in constructor
    Given WriteTool is being instantiated
    Then the constructor signature MUST be WriteTool::new(session_id: Uuid)
    And calling WriteTool::new() without session_id MUST fail to compile
    And WriteTool MUST NOT implement Default trait

  Scenario: EditTool REQUIRES session_id parameter in constructor
    Given EditTool is being instantiated
    Then the constructor signature MUST be EditTool::new(session_id: Uuid)
    And calling EditTool::new() without session_id MUST fail to compile
    And EditTool MUST NOT implement Default trait

  Scenario: GrepTool REQUIRES session_id parameter in constructor
    Given GrepTool is being instantiated
    Then the constructor signature MUST be GrepTool::new(session_id: Uuid)
    And calling GrepTool::new() without session_id MUST fail to compile
    And GrepTool MUST NOT implement Default trait

  Scenario: GlobTool REQUIRES session_id parameter in constructor
    Given GlobTool is being instantiated
    Then the constructor signature MUST be GlobTool::new(session_id: Uuid)
    And calling GlobTool::new() without session_id MUST fail to compile
    And GlobTool MUST NOT implement Default trait

  Scenario: LsTool REQUIRES session_id parameter in constructor
    Given LsTool is being instantiated
    Then the constructor signature MUST be LsTool::new(session_id: Uuid)
    And calling LsTool::new() without session_id MUST fail to compile
    And LsTool MUST NOT implement Default trait

  Scenario: AstGrepTool REQUIRES session_id parameter in constructor
    Given AstGrepTool is being instantiated
    Then the constructor signature MUST be AstGrepTool::new(session_id: Uuid)
    And calling AstGrepTool::new() without session_id MUST fail to compile
    And AstGrepTool MUST NOT implement Default trait

  Scenario: AstGrepRefactorTool REQUIRES session_id parameter in constructor
    Given AstGrepRefactorTool is being instantiated
    Then the constructor signature MUST be AstGrepRefactorTool::new(session_id: Uuid)
    And calling AstGrepRefactorTool::new() without session_id MUST fail to compile
    And AstGrepRefactorTool MUST NOT implement Default trait

  Scenario: WebSearchTool REQUIRES session_id parameter in constructor
    Given WebSearchTool is being instantiated
    Then the constructor signature MUST be WebSearchTool::new(session_id: Uuid)
    And calling WebSearchTool::new() without session_id MUST fail to compile
    And WebSearchTool MUST NOT implement Default trait

  Scenario: SearchToolFacadeWrapper REQUIRES session_id parameter in constructor
    Given SearchToolFacadeWrapper is being instantiated
    Then the constructor signature MUST be SearchToolFacadeWrapper::new(session_id: Uuid)
    And calling SearchToolFacadeWrapper::new() without session_id MUST fail to compile
    And SearchToolFacadeWrapper MUST NOT implement Default trait

  Scenario: LsToolFacadeWrapper REQUIRES session_id parameter in constructor
    Given LsToolFacadeWrapper is being instantiated
    Then the constructor signature MUST be LsToolFacadeWrapper::new(session_id: Uuid)
    And calling LsToolFacadeWrapper::new() without session_id MUST fail to compile
    And LsToolFacadeWrapper MUST NOT implement Default trait

  # ============================================================================
  # REQUIREMENT 3: PROVIDER COMPLIANCE - ALL PROVIDERS MUST PASS session_id
  # ============================================================================
  # These scenarios verify that ALL providers pass session_id to ALL tools.
  # If any provider fails to pass session_id, compilation MUST fail.
  # ============================================================================

  Scenario: Claude provider MUST pass session_id to all tools
    Given a Claude provider creating an agent with session_id
    When tools are instantiated for the agent
    Then ReadTool MUST be constructed with ReadTool::new(session_id)
    And WriteTool MUST be constructed with WriteTool::new(session_id)
    And EditTool MUST be constructed with EditTool::new(session_id)
    And GrepTool MUST be constructed with GrepTool::new(session_id)
    And GlobTool MUST be constructed with GlobTool::new(session_id)
    And LsTool MUST be constructed with LsTool::new(session_id)
    And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)

  Scenario: OpenAI provider MUST pass session_id to all tools
    Given an OpenAI provider creating an agent with session_id
    When tools are instantiated for the agent
    Then ReadTool MUST be constructed with ReadTool::new(session_id)
    And WriteTool MUST be constructed with WriteTool::new(session_id)
    And EditTool MUST be constructed with EditTool::new(session_id)
    And GrepTool MUST be constructed with GrepTool::new(session_id)
    And GlobTool MUST be constructed with GlobTool::new(session_id)
    And LsTool MUST be constructed with LsTool::new(session_id)
    And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)

  Scenario: Codex provider MUST pass session_id to all tools
    Given a Codex provider creating an agent with session_id
    When tools are instantiated for the agent
    Then ReadTool MUST be constructed with ReadTool::new(session_id)
    And WriteTool MUST be constructed with WriteTool::new(session_id)
    And EditTool MUST be constructed with EditTool::new(session_id)
    And GrepTool MUST be constructed with GrepTool::new(session_id)
    And GlobTool MUST be constructed with GlobTool::new(session_id)
    And LsTool MUST be constructed with LsTool::new(session_id)
    And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)

  Scenario: Gemini provider MUST pass session_id to all tools
    Given a Gemini provider creating an agent with session_id
    When tools are instantiated for the agent
    Then SearchToolFacadeWrapper MUST be constructed with SearchToolFacadeWrapper::new(session_id)
    And LsToolFacadeWrapper MUST be constructed with LsToolFacadeWrapper::new(session_id)
    And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)
    And all file operation wrappers MUST receive session_id

  Scenario: ZAI provider MUST pass session_id to all tools
    Given a ZAI provider creating an agent with session_id
    When tools are instantiated for the agent
    Then SearchToolFacadeWrapper MUST be constructed with SearchToolFacadeWrapper::new(session_id)
    And LsToolFacadeWrapper MUST be constructed with LsToolFacadeWrapper::new(session_id)
    And AstGrepTool MUST be constructed with AstGrepTool::new(session_id)
    And AstGrepRefactorTool MUST be constructed with AstGrepRefactorTool::new(session_id)
    And all file operation wrappers MUST receive session_id

  # ============================================================================
  # HAPPY PATH - Relative paths resolve to worktree
  # ============================================================================

  Scenario: ReadTool resolves relative path to worktree in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    And a file exists at ".fspec/worktrees/abc123/src/file.rs"
    When ReadTool reads "src/file.rs"
    Then it should read from ".fspec/worktrees/abc123/src/file.rs"

  Scenario: WriteTool writes to worktree path in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When WriteTool writes to "src/new.rs"
    Then the file should be created at ".fspec/worktrees/abc123/src/new.rs"
    And the main project directory should be unchanged

  Scenario: EditTool edits file in worktree in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    And a file exists at ".fspec/worktrees/abc123/src/lib.rs"
    When EditTool edits "src/lib.rs"
    Then it should modify ".fspec/worktrees/abc123/src/lib.rs"
    And the main project file should be unchanged

  Scenario: GrepTool searches within worktree in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When GrepTool searches with path "src/"
    Then it should search within ".fspec/worktrees/abc123/src/"
    And it should not search the main project directory

  Scenario: GlobTool finds files only within worktree in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When GlobTool searches for "**/*.rs"
    Then it should only return files within ".fspec/worktrees/abc123/"
    And it should not return files from the main project

  Scenario: LsTool lists worktree root in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When LsTool lists "."
    Then it should list contents of ".fspec/worktrees/abc123/"

  Scenario: AstGrepTool searches within worktree in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When AstGrepTool searches with path "src/"
    Then it should search within ".fspec/worktrees/abc123/src/"

  Scenario: AstGrepRefactorTool modifies worktree copy only
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    And a file exists at ".fspec/worktrees/abc123/src/lib.rs"
    When AstGrepRefactorTool refactors "src/lib.rs"
    Then it should modify ".fspec/worktrees/abc123/src/lib.rs"
    And the main project file should be unchanged

  Scenario: BashTool runs in worktree directory in isolated session
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When BashTool runs "pwd"
    Then the output should contain ".fspec/worktrees/abc123"

  # ============================================================================
  # ERROR CASES - Absolute paths outside worktree rejected
  # ============================================================================

  Scenario: ReadTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When ReadTool attempts to read "/project/src/main.rs"
    Then it should return ToolError::Validation with tool "read"
    And the error message should contain "outside isolated worktree"

  Scenario: WriteTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When WriteTool attempts to write to "/project/src/new.rs"
    Then it should return ToolError::Validation with tool "write"
    And the error message should contain "outside isolated worktree"

  Scenario: EditTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When EditTool attempts to edit "/project/src/lib.rs"
    Then it should return ToolError::Validation with tool "edit"
    And the error message should contain "outside isolated worktree"

  Scenario: GrepTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When GrepTool attempts to search with path "/project/src"
    Then it should return ToolError::Validation with tool "grep"
    And the error message should contain "outside isolated worktree"

  Scenario: GlobTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When GlobTool attempts to search with path "/project/src"
    Then it should return ToolError::Validation with tool "glob"
    And the error message should contain "outside isolated worktree"

  Scenario: LsTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When LsTool attempts to list "/project/src"
    Then it should return ToolError::Validation with tool "ls"
    And the error message should contain "outside isolated worktree"

  Scenario: AstGrepTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When AstGrepTool attempts to search with path "/project/src"
    Then it should return ToolError::Validation with tool "ast_grep"
    And the error message should contain "outside isolated worktree"

  Scenario: AstGrepRefactorTool rejects absolute path outside worktree
    Given an isolated session with worktree at ".fspec/worktrees/abc123/"
    When AstGrepRefactorTool attempts to refactor "/project/src/lib.rs"
    Then it should return ToolError::Validation with tool "ast_grep_refactor"
    And the error message should contain "outside isolated worktree"

  # ============================================================================
  # NON-ISOLATED SESSION - Normal operation
  # ============================================================================

  Scenario: Tools operate normally with Uuid::nil() in tests
    Given a non-isolated session with Uuid::nil()
    When get_effective_cwd is called
    Then it should return None
    And tools should operate in the current directory without path validation
