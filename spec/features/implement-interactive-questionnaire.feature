@questionnaire
@discovery
@cli
@phase1
@FOUND-003
Feature: Implement Interactive Questionnaire

  """
  Questionnaire must guide AI through structured discovery workflow with specific question templates
  Section 1: Vision Questions - 'What is the core purpose?', 'Who are the primary users?', 'What problem does this solve?'
  Section 2: Problem Space - 'What are the top 3-5 pain points?', 'What is the impact?', 'What is the frequency?'
  Section 3: Solution Space - 'What are the 3-7 key capabilities?', 'What makes this solution unique?', 'What is out of scope?'
  Section 4: Architecture - 'What is the tech stack?' (optional if detected), 'What are system boundaries?', 'What are external dependencies?'
  Section 5: Constraints - 'Business constraints?', 'Technical constraints?', 'Timeline/budget constraints?'
  Prefill mode: If code analysis detected values, show as defaults with [DETECTED] tag, allow AI to confirm/edit/skip
  AI guidance: Each question includes HELP text explaining WHY it's asked and EXAMPLES of good answers
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Questions must be grouped by section with progress indicator (Question 3 of 15)
  #   2. Must support both prefilled mode (--from-discovery) and blank slate (--interactive)
  #   3. Must validate answers before accepting (e.g. non-empty strings, valid format)
  #
  # EXAMPLES:
  #   1. Vision question with help: 'What is the core purpose? [HELP: One sentence elevator pitch. Example: fspec helps AI agents follow ACDD workflow]'
  #   2. Prefilled answer: 'Primary user: Developer using CLI [DETECTED from code analysis] - Keep/Edit/Skip?'
  #
  # ========================================

  Background: User Story
    As a developer using fspec with AI to bootstrap foundation documents
    I want to gather WHY/WHAT information through structured questionnaire
    So that I can create complete foundation.json without manual analysis

  Scenario: Display vision question with help text and example
    Given I run questionnaire in interactive mode
    When questionnaire displays vision section question
    Then question should show 'What is the core purpose?'
    And question should include HELP text with elevator pitch guidance
    And question should include example answer


  Scenario: Display prefilled answer with DETECTED tag from code analysis
    Given I run questionnaire with --from-discovery option
    And code analysis detected primary user as 'Developer using CLI'
    When questionnaire displays personas question
    Then answer should be prefilled with detected value
    And answer should show [DETECTED] tag
    And user should have Keep/Edit/Skip options


  Scenario: Show progress indicator for each question
    Given I am answering the third question out of 15 total
    When questionnaire displays the question
    Then UI should show 'Question 3 of 15'
    And progress indicator should update with each question


  Scenario: Validate non-empty answer before accepting
    Given I am answering a required question
    When I submit an empty answer
    Then questionnaire should reject the answer
    And questionnaire should display validation error message
    And questionnaire should prompt me to answer again

