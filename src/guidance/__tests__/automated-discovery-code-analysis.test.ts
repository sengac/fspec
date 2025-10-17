/**
 * Feature: spec/features/implement-automated-discovery-code-analysis.feature
 *
 * This test file validates the acceptance criteria for automated discovery guidance.
 * Tests map directly to scenarios in the Gherkin feature.
 *
 * IMPORTANT: This feature is about GUIDANCE for AI, not code implementation.
 * Tests verify that guidance documentation exists and contains correct prompts/patterns.
 */

import { describe, it, expect } from 'vitest';
import { discoveryGuidance } from '../automated-discovery-code-analysis';

describe('Feature: Implement Automated Discovery - Code Analysis', () => {
  describe('Scenario: Discover CLI tool project type and developer persona', () => {
    it('should have guidance for discovering CLI tools from commander.js', () => {
      // Given I have guidance documentation for discovering CLI tools
      const guidance = discoveryGuidance.cliTools;

      // When AI analyzes codebase with commander.js command structure
      expect(guidance).toBeDefined();
      expect(guidance.pattern.indicators).toContain('commander');

      // Then AI should infer project type as 'cli-tool'
      expect(guidance.inference.projectType).toBe('cli-tool');

      // And AI should infer persona 'Developer using CLI in terminal'
      expect(guidance.inference.persona).toBe(
        'Developer using CLI in terminal'
      );
    });
  });

  describe('Scenario: Discover web app project type with multiple personas', () => {
    it('should have guidance for discovering web apps from Express and React', () => {
      // Given I have guidance documentation for discovering web applications
      const guidance = discoveryGuidance.webApps;

      // When AI analyzes codebase with Express routes and React components
      expect(guidance).toBeDefined();
      expect(guidance.pattern.indicators).toContain('express');
      expect(guidance.pattern.indicators).toContain('React.Component');

      // Then AI should infer project type as 'web-app'
      expect(guidance.inference.projectType).toBe('web-app');

      // And AI should infer persona 'End User' from UI components
      const endUserPersona = guidance.inference.personas.find(
        (p: { name: string }) => p.name === 'End User'
      );
      expect(endUserPersona).toBeDefined();
      expect(endUserPersona?.source).toBe('UI components');

      // And AI should infer persona 'API Consumer' from API routes
      const apiConsumerPersona = guidance.inference.personas.find(
        (p: { name: string }) => p.name === 'API Consumer'
      );
      expect(apiConsumerPersona).toBeDefined();
      expect(apiConsumerPersona?.source).toBe('API routes');
    });
  });

  describe('Scenario: Discover library project type from package exports', () => {
    it('should have guidance for discovering libraries from package.json exports', () => {
      // Given I have guidance documentation for discovering libraries
      const guidance = discoveryGuidance.libraries;

      // When AI analyzes codebase with package.json exports field
      expect(guidance).toBeDefined();
      expect(guidance.pattern.indicators).toContain(
        'exports field in package.json'
      );

      // Then AI should infer project type as 'library'
      expect(guidance.inference.projectType).toBe('library');

      // And AI should infer persona 'Developer integrating library into their codebase'
      expect(guidance.inference.persona).toBe(
        'Developer integrating library into their codebase'
      );
    });
  });

  describe('Scenario: Infer capabilities focusing on WHAT not HOW', () => {
    it('should have guidance for inferring high-level capabilities, not implementation', () => {
      // Given I have guidance documentation for capability inference
      const guidance = discoveryGuidance.capabilities;

      // When AI analyzes React components for user features
      expect(guidance).toBeDefined();
      expect(guidance.principle).toContain('WHAT');
      expect(guidance.principle).toContain('not HOW');

      // Then AI should infer capability 'User Interface' not 'Uses React hooks'
      expect(guidance.examples.good).toContain('User Interface (WHAT)');
      expect(guidance.examples.bad).toContain('Uses React hooks (HOW)');

      // And AI should focus on high-level features not implementation details
      expect(guidance.prompt).toContain('WHAT the system does');
      expect(guidance.prompt).toContain('not HOW');
    });
  });

  describe('Scenario: Infer problems as WHY not implementation details', () => {
    it('should have guidance for inferring user problems, not technical needs', () => {
      // Given I have guidance documentation for problem inference
      const guidance = discoveryGuidance.problems;

      // When AI discovers a React app codebase
      expect(guidance).toBeDefined();
      expect(guidance.principle).toContain('WHY');
      expect(guidance.principle).toContain('not technical implementation');

      // Then AI should infer problem 'Users need interactive web UI' not 'Code needs React'
      expect(guidance.examples.good).toContain(
        'Users need interactive web UI (WHY)'
      );
      expect(guidance.examples.bad).toContain('Code needs React (HOW)');

      // And AI should focus on user needs not technical implementation choices
      expect(guidance.prompt).toContain('USER NEEDS');
      expect(guidance.prompt).toContain('not technical implementation');
    });
  });
});
