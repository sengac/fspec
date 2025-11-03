@feature-management
@cli-integration
@done
@ai-assisted
@github-integration
@browser-integration
@bug-reporting
@cli
@high
@CLI-014
Feature: Report bug to GitHub with AI assistance
  """
  Uses browser launcher utility (openBrowser) adapted from cage project with 'open' npm package. GitHub issue URL format: https://github.com/owner/repo/issues/new?title={title}&body={body}&labels={labels} with proper URL encoding. AI-driven interactive analysis pattern similar to 'fspec review' command. Gathers system context (version, platform, git status, work unit context) automatically before prompting user.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Bug reports must include system context (fspec version, node version, OS, platform)
  #   2. All content must be properly URL-encoded for GitHub query parameters
  #   3. Browser must be accessible to open GitHub issue creation page
  #   4. Markdown formatting must be preserved when rendered in GitHub
  #   5. Default labels should be 'bug,needs-triage' for new bug reports
  #   6. Users must confirm before opening browser
  #   7. Work unit context should be included if user is working on a work unit
  #   8. Git context (branch, uncommitted changes) should be included if available
  #
  # EXAMPLES:
  #   1. User runs 'fspec report-bug-to-github', answers AI questions about the bug, sees formatted preview, confirms, and browser opens to GitHub with pre-filled issue form
  #   2. User working on AUTH-001 encounters bug, runs report command, generated issue includes work unit ID, title, status, and link to feature file
  #   3. User with uncommitted git changes reports bug, issue includes current branch name and note about uncommitted changes
  #   4. fspec command crashes with ENOENT error, user runs report-bug-to-github, command detects recent error log and includes stack trace in bug report
  #   5. AI generates bug report preview, user decides to edit title, command allows editing, user confirms, browser opens with edited content
  #   6. Bug report contains code with special characters (", &, <, >) and markdown code blocks, GitHub URL properly encodes everything, markdown renders correctly in GitHub issue
  #
  # ========================================
  Background: User Story
    As a developer using fspec who encounters a bug
    I want to report it to GitHub through an interactive guided process
    So that I can provide comprehensive bug information without manually writing issue reports

  Scenario: Basic bug report flow
    Given I am in a project using fspec
    And I have encountered a bug
    When I run "fspec report-bug-to-github"
    Then the command should gather system context automatically
    And the command should prompt me interactively for bug details
    And the command should generate a complete bug report with markdown formatting
    And the command should display a preview of the issue
    And the command should ask for confirmation
    And the command should open my browser to GitHub with pre-filled issue

  Scenario: Include work unit context
    Given I am working on work unit AUTH-001
    And the bug occurs while implementing AUTH-001
    When I run "fspec report-bug-to-github"
    Then the bug report should include the work unit ID "AUTH-001"
    And the bug report should include the work unit title
    And the bug report should include the current work unit status
    And the bug report should include a link to the related feature file if it exists

  Scenario: Include git context
    Given I have uncommitted changes in my working directory
    And the bug reproduces with these changes
    When I run "fspec report-bug-to-github"
    Then the bug report should include the current git branch
    And the bug report should indicate there are uncommitted changes
    And the bug report should note about providing git diff if needed

  Scenario: Error log capture
    Given fspec recently crashed with an error
    When I run "fspec report-bug-to-github"
    Then the command should detect recent error logs
    And the command should include the stack trace if available
    And the command should include the error message in the bug report

  Scenario: Preview and edit
    Given the AI has generated a bug report
    When I review the preview
    And I want to add additional context
    Then I should be able to edit the title before submission
    And I should be able to edit the body before submission
    And I should be able to cancel the submission
    And I should be able to confirm and open the browser

  Scenario: URL encoding handling
    Given the bug report contains special characters
    And the report contains markdown code blocks
    When constructing the GitHub URL
    Then all content should be properly URL-encoded
    And markdown formatting should be preserved
    And code blocks should render correctly in GitHub
