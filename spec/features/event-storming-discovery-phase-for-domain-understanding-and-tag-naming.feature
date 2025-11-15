@medium
@ddd
@tag-management
@cli
@discovery-workflow
@EXMAP-004
Feature: Event Storming discovery phase for domain understanding and tag naming
  """
  Transformation pipeline: Event Storm artifacts → Tag discovery → Enhanced tags.json with relationships (hierarchical, semantic, domain)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Event Storming uses colored sticky notes to represent different concepts: Orange=Domain Events, Blue=Commands, Yellow=Aggregates, Pink=External Systems, Purple=Policies, Green=Read Models, Red=Hotspots/Questions
  #   2. Event Storming happens BEFORE Example Mapping in the discovery workflow
  #   3. Event Storming identifies domain events (things that happened in past tense) which can inform tag naming and bounded contexts
  #   4. Current fspec workflow has no Event Storming support - discovery goes straight to Example Mapping
  #   5. Event Storming data stored in both foundation.json (Big Picture ES with bounded contexts) and work-units.json (Process/Design ES per story). Tags.json gets enhanced relationships section.
  #   6. Yes, fspec commands: add-domain-event, add-command, add-aggregate, add-policy, add-hotspot, add-external-system, add-bounded-context, suggest-tags-from-events, sync-tags-with-event-storm, add-tag-relationship, show-tag-relationships
  #   7. Yes, domain events visible during Example Mapping via transformation (policies→rules, hotspots→questions). Also used for tag suggestions when generating scenarios.
  #   8. Both. Big Picture ES done at foundation level (foundation.json) for strategic bounded contexts. Process/Design ES done per epic/feature (work-units.json) for tactical discovery.
  #   9. Track all: domain events, commands, aggregates, policies, external systems, bounded contexts, hotspots. This provides complete domain model for tag discovery and transformation to Example Mapping.
  #
  # EXAMPLES:
  #   1. Team runs Event Storming session, identifies domain events like 'UserRegistered', 'CheckpointCreated', 'WorkUnitStatusChanged', then creates tags based on these events and bounded contexts
  #   2. AI runs Event Storming discovery command, asks questions about domain events, captures events in structured format, suggests tag names based on event clusters and bounded contexts
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should Event Storming data be stored? New section in foundation.json, separate event-storm.json file, or in work units themselves?
  #   A: true
  #
  #   Q: Should there be fspec commands for Event Storming? E.g., fspec add-domain-event, fspec add-command, fspec add-aggregate, fspec suggest-tags-from-events?
  #   A: true
  #
  #   Q: How should Event Storming integrate with Example Mapping? Should domain events from Event Storming be visible when doing Example Mapping for a story?
  #   A: true
  #
  #   Q: Should Event Storming be part of foundation discovery (done once at project start) or per-epic/feature discovery (done multiple times)?
  #   A: true
  #
  #   Q: What Event Storming elements should be tracked? Just domain events and commands, or also aggregates, policies, external systems, hotspots?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for domain-driven development
    I want to perform Event Storming discovery before Example Mapping
    So that I identify bounded contexts, aggregates, and domain events that inform tag naming and domain structure

  Scenario: Capture domain events and create tags from Event Storm session
    Given I have a work unit in specifying status
    When I run Event Storming and identify domain events "UserRegistered", "CheckpointCreated", "WorkUnitStatusChanged"
    And I add these events using "fspec add-domain-event" command
    And I identify bounded contexts based on event clusters
    And I run "fspec suggest-tags-from-events" to analyze Event Storm data
    Then component tags should be suggested based on bounded contexts and aggregates
    And feature group tags should be suggested based on domain event clusters
    And I can register these tags using "fspec sync-tags-with-event-storm"

  Scenario: AI-guided Event Storming discovery with structured capture
    Given I have a work unit ready for Event Storming discovery
    When I run Event Storming discovery commands
    And I add domain events, commands, and aggregates to the work unit
    And I capture policies, hotspots, and external systems
    And I define bounded context boundaries
    Then all Event Storm artifacts are stored in work-units.json eventStorm section
    And tag suggestions are generated based on domain model analysis
    And tag relationships are created (hierarchical, semantic, domain mappings)
    And policies are transformed to Example Mapping rules
    And hotspots are transformed to Example Mapping questions

  Scenario: Foundation-level Big Picture Event Storming
    Given I am bootstrapping a new project with fspec
    When I perform Big Picture Event Storming to identify strategic bounded contexts
    And I add bounded contexts, major aggregates, and pivotal events to foundation.json
    Then the foundation Event Storm data defines the overall domain architecture
    And component tags are derived from bounded contexts
    And feature group tags are derived from major domain event categories
    And tags.json receives enhanced relationships section linking tags to domain concepts
