@wip
@bug-fix
@rust
@tools
@blocklist
@security
@BUG-644
Feature: Blocklist silently allows or skips: a sensitive-path check allows any unexpected pause response, and invalid regex rules are skipped
  """
  Reporting: load_blocklist_config returns the merged config together with the load problems (an unreadable or unparsable system or project file, an unresolvable project state root); BlocklistMatcher::new returns the matcher together with one problem per invalid rule. check_bash_command, check_file_path and check_file_path_semantic report each problem to the session that ran the check through a new codelet-tools facade callback (registered in fspec common::build_service) that sends the session a user notification with severity Error, once per session per distinct problem text, and log it with tracing::error!. rpc-types already has NotificationSeverity::Error: no wire change.
  Git identity: merge_commit.rs uses gix's configured identity (repo.committer(): the repository, global and system git config plus GIT_COMMITTER_* the way git reads them) instead of parsing the repo-local config file; no identity anywhere is GitError naming user.name and user.email. Tests isolate git's global and system config (HOME, XDG_CONFIG_HOME, GIT_CONFIG_NOSYSTEM) and never read the user's ~/.gitconfig.
  Pause answers: the blocklist command and file prompts are PauseKind::Triple; only AllowOnce, AllowSession, Denied and Interrupted answer it. Resumed (Continue's answer, and pause_for_user's answer when the session has no handler) and Approved (Confirm's answer) refuse with 'Blocked: the blocklist prompt for <command or path> got the answer <Answer>, which does not answer an allow/deny prompt'. The RLCD gate (rlcd/gate.rs) and RLCD security prompt (rlcd/security.rs) also treat Resumed as allowed; they are outside this card.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The blocklist-core tests assert the exact error text their scenarios state, not fragments
  #   2. A blocklist prompt, for a command or a sensitive file, answered with anything but allow once, allow for the session, deny or an interrupt refuses the command or access with an exact error naming the answer and the command or path; this includes Resumed, which pause_for_user also answers when the session has no prompt handler, and Approved
  #   3. A blocklist rule whose pattern is not a valid regular expression is reported as an error the user sees, naming the rule, its pattern and the regex error; only that rule is skipped and every valid rule is still enforced (product owner, 2026-11-09: not fail-closed)
  #   4. A project state root that cannot be resolved (a broken .fspec/state-root pointer, WT-015) is reported as an error the user sees with the exact pointer error; the system blocklist is still enforced (product owner, 2026-11-09)
  #   5. A system or project blocklist file that cannot be read or parsed is reported as an error the user sees, naming the file and the read or parse error; every blocklist that does load is still enforced (product owner, 2026-11-09)
  #   6. Merging a worktree commit (git merge_commit.rs resolve_identity) reads user.name and user.email from the repository, global and system git config the way git does, never falls back to a built-in 'fspec' identity, and fails with an exact missing-identity error only when no identity is set anywhere (product owner, 2026-11-09)
  #
  # EXAMPLES:
  #   1. Rule env-file-prompt asks before reading .env; the prompt is answered Resumed: the read is refused with the exact error naming Resumed and the .env path
  #   2. Rule git-checkout-block with reason 'git checkout is deprecated.' and guidance 'Use git switch main instead.': running 'git checkout main' gives exactly 'Blocked: git checkout is deprecated. Use git switch main instead.'
  #   3. The blocklist has rule bad-rule with pattern '(' and rule git-checkout-block: running 'ls' runs, the user sees an error naming bad-rule, '(' and the regex error, and 'git checkout main' is still blocked by git-checkout-block
  #   4. The project's .fspec/state-root pointer names a missing directory: the user sees the exact pointer error and a command blocked by the system blocklist is still blocked
  #   5. ~/.fspec/blocklist.json holds '{not json' and the project blocklist blocks 'make deploy': the user sees an error naming ~/.fspec/blocklist.json and the parse error, and 'make deploy' is still blocked
  #   6. Rule npm-install-prompt asks before 'npm install' and the session has no prompt handler (pause_for_user answers Resumed): 'npm install' is refused with the exact error naming Resumed and the command
  #   7. A worktree merge in a repository with no local identity, where the user's global git config sets user.name and user.email, commits as that global identity; with no identity in the repository, global or system config it fails with the exact missing-identity error and nothing is committed
  #
  # ========================================
  Background: User Story
    As a user whose agent runs under the blocklist
    I want to have the blocklist refuse, with an exact error, every check it cannot decide
    So that a sensitive file is never opened on an unexpected prompt answer and a broken rule never silently stops protecting me

  Scenario: A sensitive-file prompt answered Resumed refuses the read
    Given rule "env-file-prompt" asks before reading ".env"
    When the AI reads "/project/.env" and the prompt is answered Resumed
    Then the read is refused with exactly 'Blocked: the blocklist prompt for "/project/.env" got the answer Resumed, which does not answer an allow/deny prompt'

  Scenario: A command prompt in a session with no prompt handler refuses the command
    Given rule "npm-install-prompt" asks before "npm install"
    And the session has no prompt handler, so pause_for_user answers Resumed
    When the AI runs "npm install"
    Then the command is refused with exactly 'Blocked: the blocklist prompt for "npm install" got the answer Resumed, which does not answer an allow/deny prompt'

  Scenario: An invalid rule is reported once and only that rule is skipped
    Given the blocklist has rule "bad-rule" with pattern "(" and rule "git-checkout-block" blocking "git checkout"
    When the AI runs "ls"
    Then "ls" runs
    And the session gets one error notification 'blocklist rule "bad-rule" has an invalid pattern "(": <the regex error>'
    When the AI runs "git checkout main"
    Then it is refused by "git-checkout-block"
    And the session gets no second notification for "bad-rule"

  Scenario: An unresolvable project state root is reported and the system blocklist is still enforced
    Given the project's ".fspec/state-root" pointer names a missing directory
    And the system blocklist blocks "make deploy"
    When the AI runs "make deploy"
    Then it is refused by the system rule
    And the session gets an error notification 'cannot resolve the blocklist config: <the exact pointer error>'

  Scenario: An unparsable system blocklist is reported and the project blocklist is still enforced
    Given the system blocklist file holds "{not json"
    And the project blocklist blocks "make deploy"
    When the AI runs "make deploy"
    Then it is refused by the project rule
    And the session gets an error notification 'cannot load the blocklist <system blocklist path>: <the parse error>'

  Scenario: A worktree merge commits as the global git identity when the repository sets none
    Given a repository with no user.name or user.email in its own config
    And the global git config sets user.name "Global Person" and user.email "global@example.com"
    When a session's worktree is merged and committed
    Then the commit's author and committer are "Global Person <global@example.com>"

  Scenario: A worktree merge with no git identity anywhere fails and commits nothing
    Given no user.name or user.email in the repository, global or system git config
    When a session's worktree is merged and committed
    Then it fails with exactly 'no git identity: set user.name and user.email (git config --global user.name "Your Name"; git config --global user.email you@example.com)'
    And HEAD is unchanged
