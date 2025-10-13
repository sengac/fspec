=== PROJECT ===
Name: fspec
Description: A standardized CLI tool for AI agents to manage Gherkin-based feature specifications and project work units using Kanban-based workflow and Acceptance Criteria Driven Development (ACDD)
Repository: https://github.com/sengac/fspec
License: MIT

=== WHAT WE ARE BUILDING ===
A Kanban-based project management and specification tool for AI agents. fspec provides AI agents (like Claude Code, GitHub Copilot) with a structured workflow for building software using Acceptance Criteria Driven Development (ACDD). It has two integrated systems: (1) Kanban Project Management - work units flow through a 7-state workflow (backlog → specifying → testing → implementing → validating → done, plus blocked), with example mapping for discovery, dependency tracking, and metrics; (2) Specification Management - validated Gherkin feature files with enforced tag discipline, architecture documentation (Mermaid diagrams), automated formatting, and coverage tracking (scenario-to-test-to-implementation mappings). Coverage files (.feature.coverage) provide full traceability from Gherkin scenarios to test files and implementation code, which is critical for reverse ACDD (tracking what's been mapped) and refactoring safety. AI agents use fspec to manage their work in Kanban fashion, discover requirements through example mapping, write specifications, track coverage, and implement features following ACDD methodology. fspec supports both forward ACDD (new features) and reverse ACDD (reverse engineering existing codebases via /rspec command in Claude Code) to create specifications from legacy code with full coverage tracking.

=== WHY WE ARE BUILDING IT ===
Problem: AI Agents Lack Structured Workflow for Building the Right Software
AI agents (like Claude Code, GitHub Copilot) excel at writing code but struggle to build quality software reliably because they lack persistent, queryable project state and structured discovery tools. They rely on fragile conversation context, flat TODO lists, and ad-hoc workflows - leading to specification drift, skipped ACDD phases, lost context between sessions, and building the wrong features.

=== ARCHITECTURE DIAGRAMS ===
- fspec System Context
- fspec Command Architecture
- ACDD Workflow with fspec
- Data and Storage Architecture
- Kanban Workflow State Machine
- Example Mapping Discovery Process
- Help System Architecture
- Coverage Tracking System
