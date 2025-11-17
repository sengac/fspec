@querying
@discovery
@high
@research
@cli
@integration
@RES-006
Feature: Stakeholder communication research script for unanswered questions
  """
  Uses plugin architecture for extensible platform support. Main script in spec/research-scripts/stakeholder with platform plugins in spec/research-scripts/plugins/. Plugins can be shell scripts, Python, JavaScript, or compiled binaries - auto-discovered via executable bit. Script is one-way notification (fire-and-forget), does not wait for responses. Reads credentials from user-level config ~/.fspec/fspec-config.json. Message format includes full Example Mapping context: work unit ID, title, epic, question, rules, examples, and previous Q&A.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Use plugin architecture where communication platforms (Teams, Slack, etc.) can be added dynamically without modifying core script
  #   2. Main script stored in spec/research-scripts/stakeholder.sh, platform plugins stored in spec/research-scripts/plugins/ directory
  #   3. Script is one-way notification only: sends message to stakeholders and exits. Does NOT wait for response or handle callbacks.
  #   4. Message sender identity should be configurable (who the message appears to come from)
  #   5. Store platform credentials and settings in user-level config: ~/.fspec/fspec-config.json (NOT project-level spec/fspec-config.json)
  #   6. Message should include full context: question text, work unit ID, work unit title, epic (if any), existing rules, existing examples, and all questions/answers so far from Example Mapping
  #   7. AI asks human which platform(s) to use. If no preference specified, default to all configured platforms in ~/.fspec/fspec-config.json
  #   8. Auto-discover platform plugins by scanning spec/research-scripts/plugins/ directory for .sh files (no manifest/registry required)
  #   9. Auto-discover platform plugins by scanning spec/research-scripts/plugins/ directory for ANY executable files (not just .sh - could be .py, .js, compiled binaries, etc.)
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=stakeholder --platform=teams --question="Should we support OAuth2?"'. Main script loads spec/research-scripts/plugins/teams.sh and sends question via Teams API.
  #   2. AI runs 'fspec research --tool=stakeholder --platform=slack --question="OAuth support needed?"'. Script sends message to configured Slack channel from configured user, then exits. Stakeholder sees notification and responds manually later in Claude Code session.
  #   3. User-level config at ~/.fspec/fspec-config.json contains Slack token and Teams webhook URL. Script reads from user config (not project config) to send stakeholder notifications.
  #   4. Message sent to Slack: 'Question for AUTH-001 (User Login, epic: user-management)\n\nQuestion: Should we support OAuth2?\n\nRules:\n1. Password must be 8+ chars\n\nExamples:\n1. Valid login with email/password\n\nPrevious Q&A:\nQ: Support 2FA? A: Yes, in phase 2'
  #   5. AI asks: 'Send to Teams, Slack, or both?' Human says 'both'. Script sends to both platforms. If human doesn't specify, script checks config and sends to all configured platforms by default.
  #   6. Plugins directory contains teams.sh and slack.sh. Main script auto-discovers both platforms and can send to either/both based on config and user preference.
  #   7. Plugins directory contains teams.py (Python), slack (compiled Go binary), and discord.js (Node script). Main script auto-discovers all three by checking executable bit, not file extension.
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should multiple stakeholders work? (A) Single destination per invocation, (B) Multiple destinations via repeated --platform flags, (C) Send to all configured platforms at once?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent or developer using fspec during Example Mapping
    I want to contact stakeholders via chat platforms when questions arise
    So that I can get human input on unclear requirements without blocking progress

  Scenario: Send question to Teams platform
    Given the stakeholder research tool exists in spec/research-scripts/
    When I run "fspec research --tool=stakeholder --platform=teams --question='Should we support OAuth2?'"
    Then the main script should load the Teams plugin
    Given the Teams plugin exists in spec/research-scripts/plugins/
    Given Teams credentials are configured in ~/.fspec/fspec-config.json
    Then the question should be sent via Teams API
    Then the script should exit without waiting for response

  Scenario: Send question to Slack platform and exit
    Given the Slack plugin exists in spec/research-scripts/plugins/
    When I run "fspec research --tool=stakeholder --platform=slack --question='OAuth support needed?'"
    Then the message should be sent to configured Slack channel
    Given Slack credentials are configured in ~/.fspec/fspec-config.json
    Then the script should exit immediately
    Then stakeholder sees notification for manual response later

  Scenario: Read credentials from user-level config
    Given ~/.fspec/fspec-config.json contains Slack token and Teams webhook URL
    When the script needs to send stakeholder notifications
    Then it should read from user config (NOT project-level spec/fspec-config.json)
    Then credentials should be loaded from ~/.fspec/fspec-config.json

  Scenario: Include full Example Mapping context in message
    Given work unit AUTH-001 has title 'User Login', epic 'user-management'
    When I send question 'Should we support OAuth2?' for work unit AUTH-001
    Then the message should include work unit ID and title
    Given work unit has rule 'Password must be 8+ chars'
    Given work unit has example 'Valid login with email/password'
    Given work unit has previous Q&A 'Q: Support 2FA? A: Yes, in phase 2'
    Then the message should include epic name
    Then the message should include the question text
    Then the message should include all rules
    Then the message should include all examples
    Then the message should include previous Q&A history

  Scenario: Send to multiple platforms with user choice
    Given Teams and Slack are both configured in ~/.fspec/fspec-config.json
    When AI asks 'Send to Teams, Slack, or both?'
    Then the script should send to Teams
    When human responds 'both'
    Then the script should send to Slack
    Then if no preference specified, default to all configured platforms

  Scenario: Auto-discover shell script plugins
    Given plugins directory contains teams.sh and slack.sh
    When the main script starts
    Then it should auto-discover both platforms by scanning directory
    Then it should be able to send to either or both based on config and user preference

  Scenario: Auto-discover plugins of any language by executable bit
    Given plugins directory contains teams.py (Python)
    When the main script scans for plugins
    Then it should discover all three plugins by checking executable bit
    Given plugins directory contains slack (compiled Go binary)
    Given plugins directory contains discord.js (Node script)
    Then it should NOT rely on file extension (.py, .js, etc.)
