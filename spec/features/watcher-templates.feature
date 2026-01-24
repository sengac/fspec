@done
@watcher
@tui
@WATCH-023
Feature: Watcher Templates and Improved Creation UX

  """
  Architecture Notes:
  - Types: src/tui/types/watcherTemplate.ts (WatcherTemplate, WatcherInstance interfaces)
  - Storage: src/tui/utils/watcherTemplateStorage.ts (uses getFspecUserDir() from src/utils/config.ts)
  - List: src/tui/components/WatcherTemplateList.tsx (collapse/expand pattern from model selector)
  - Form: Refactor WatcherCreateView.tsx → WatcherTemplateForm.tsx (create/edit modes)
  - Integration: Replace watcher overlay in AgentView.tsx, add /watcher spawn command
  - Dialogs: Reuse ConfirmationDialog from src/components/ConfirmationDialog.tsx
  - Feedback: Use NotificationDialog (success) and ErrorDialog (errors) - NOT setConversation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /watcher overlay shows templates as primary list (not active instances)
  #   2. Active instances nest under parent template with collapse/expand
  #   3. Templates stored at ~/.fspec/watcher-templates.json (via getFspecUserDir)
  #   4. Enter on template = spawn new instance
  #   5. Enter on instance = open that watcher session
  #   6. Up/Down arrows navigate flat list (templates + visible instances)
  #   7. Left arrow collapses template, Right arrow expands (if has instances)
  #   8. Model field shows only configured models (with API keys)
  #   9. Model defaults to parent session's current model
  #   10. Model selection uses type-to-filter (not arrow scrolling)
  #   11. Up/Down arrows navigate form fields (in addition to Tab)
  #   12. Authority (Peer/Supervisor) shown in template and instance lists
  #   13. Authority field shows inline explanation when focused
  #   14. E key edits selected template
  #   15. D on template = delete with confirmation (auto-kills instances with warning)
  #   16. D on instance = kill that instance
  #   17. N key opens template creation form
  #   18. Template contains: name, modelId, authority, brief, autoInject
  #   19. Empty state explains watchers and prompts to create first template
  #   20. Templates listed alphabetically with type-to-filter search
  #   21. Auto-generated slug in kebab-case (Security Reviewer → security-reviewer)
  #   22. /watcher spawn <slug> command for quick spawning
  #   23. Success actions show NotificationDialog (auto-dismiss in 2s)
  #   24. Error actions show ErrorDialog (ESC to dismiss)
  #   25. Feedback uses dialogs, NOT setConversation status messages
  #   26. Switching between sessions (watcher/parent) does NOT show notification (UI change is feedback)
  #
  # ========================================

  Background: User Story
    As a TUI user managing watcher sessions
    I want to create, manage, and spawn watchers from reusable templates
    So that I can quickly spawn watchers without filling out forms from scratch each time

  # ===========================================
  # Template List - Display and Navigation
  # ===========================================

  Scenario: View templates with active instance count
    Given I have a "Security Reviewer" template with 2 active instances
    And I have a "Test Enforcer" template with no active instances
    When I open the /watcher overlay
    Then I should see "Security Reviewer (Supervisor)" with "[2 active]"
    And I should see "Test Enforcer (Peer)" without an active badge
    And templates should be in alphabetical order

  Scenario: Expand template to show instances
    Given "Security Reviewer" template has 2 active instances and is collapsed
    When I press the right arrow key
    Then the template should expand
    And I should see "#1 running" nested with tree connector
    And I should see "#2 idle" nested with tree connector

  Scenario: Collapse template to hide instances
    Given "Security Reviewer" template is expanded showing instances
    When I press the left arrow key
    Then the template should collapse
    And instances should be hidden

  Scenario: Navigate list with arrow keys
    Given I have multiple templates in the /watcher overlay
    When I press the down arrow key
    Then selection moves to the next item
    When I press the up arrow key
    Then selection moves to the previous item

  Scenario: Filter templates by typing
    Given I have templates including "Security Reviewer" and "Test Enforcer"
    When I type "sec" in the overlay
    Then only "Security Reviewer" should be visible
    And "Test Enforcer" should be hidden

  Scenario: Empty state with no templates
    Given I have no watcher templates
    When I open the /watcher overlay
    Then I should see "No watcher templates yet"
    And I should see an explanation of what watchers do
    And I should see "Press N to create your first template"

  # ===========================================
  # Spawning and Opening Watchers
  # ===========================================

  Scenario: Spawn new instance from template
    Given "Security Reviewer" template is selected
    When I press Enter
    Then a new watcher instance spawns with the template's settings
    And the active count badge updates accordingly
    And a success notification shows "Spawned watcher"

  Scenario: Open existing watcher instance
    Given "Security Reviewer" is expanded with instance "#1 running" selected
    When I press Enter
    Then the overlay closes
    And I switch to that watcher's session

  Scenario: Quick spawn via slash command
    Given I have a "Security Reviewer" template with slug "security-reviewer"
    When I type "/watcher spawn security-reviewer"
    Then a watcher instance spawns immediately without opening the overlay
    And a success notification shows "Spawned watcher"

  Scenario: Quick spawn with unknown slug shows error
    Given no template exists with slug "unknown-watcher"
    When I type "/watcher spawn unknown-watcher"
    Then an error dialog shows "No template found with slug: unknown-watcher"

  # ===========================================
  # Template CRUD Operations
  # ===========================================

  Scenario: Create new template
    Given the /watcher overlay is open
    When I press N
    Then the template form opens in create mode
    And all fields are empty except Model which defaults to parent's model

  Scenario: Edit existing template
    Given "Security Reviewer" template is selected
    When I press E
    Then the template form opens in edit mode
    And all fields are pre-populated from the template

  Scenario: Delete template without active instances
    Given "Architecture Advisor" template has no active instances and is selected
    When I press D
    Then a confirmation dialog appears
    When I confirm
    Then the template is deleted
    And a success notification shows "Deleted template"

  Scenario: Delete template with active instances shows warning
    Given "Security Reviewer" template has 2 active instances and is selected
    When I press D
    Then a confirmation dialog warns "This will kill 2 active watchers"
    When I confirm
    Then all active instances are killed
    And the template is deleted
    And a success notification shows "Deleted template"

  Scenario: Kill watcher instance
    Given "Security Reviewer" is expanded with instance "#2 idle" selected
    When I press D
    Then the instance is killed
    And it disappears from the list
    And the active count badge decreases
    And a success notification shows "Killed watcher instance"

  # ===========================================
  # Template Form - Navigation
  # ===========================================

  Scenario: Navigate form fields with arrow keys
    Given the template form is open with Name field focused
    When I press the down arrow key
    Then focus moves to Model field
    When I press the up arrow key
    Then focus moves back to Name field

  Scenario: Navigate form fields with Tab
    Given the template form is open
    When I press Tab repeatedly
    Then focus cycles through: Name, Model, Authority, Brief, Auto-inject

  # ===========================================
  # Template Form - Model Selection
  # ===========================================

  Scenario: Model defaults to parent session model
    Given my parent session uses "claude-sonnet-4"
    When I open the template form
    Then Model field shows "claude-sonnet-4"

  Scenario: Filter models by typing
    Given the Model field is focused
    And configured models include "gemini-2.0-flash" and "claude-sonnet-4"
    When I type "gem"
    Then only "gemini-2.0-flash" appears in the filtered list

  Scenario: Only configured models are shown
    Given I have API keys for "anthropic" but not "openai"
    When I view the Model field options
    Then I see Anthropic models
    And I do not see OpenAI models

  # ===========================================
  # Template Form - Authority Field
  # ===========================================

  Scenario: Authority shows inline explanation when focused
    Given the template form is open
    When I focus the Authority field
    Then I see "Peer: Suggestions the AI can consider or ignore"
    And I see "Supervisor: Directives the AI should follow"

  Scenario: Toggle authority with arrow keys
    Given Authority field is focused showing "Peer"
    When I press the right arrow key
    Then Authority changes to "Supervisor"
    When I press the left arrow key
    Then Authority changes to "Peer"

  # ===========================================
  # Template Slug Generation
  # ===========================================

  Scenario: Slug is auto-generated from template name
    When I save a template with name "Security Reviewer"
    Then the template slug is "security-reviewer"

  Scenario: Slug handles special characters
    When I save a template with name "Code Review & Analysis"
    Then the template slug is "code-review-analysis"
