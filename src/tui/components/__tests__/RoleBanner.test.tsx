/**
 * Feature: spec/features/role-banner-display.feature
 *
 * Tests for RoleBanner component — displays active role text
 * below SessionHeader in AgentView.
 *
 * TUI-081: Display active role as pinned RoleBanner in AgentView below SessionHeader
 */

import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { RoleBanner } from '../RoleBanner';

describe('Feature: Role banner display in AgentView', () => {
  // ============================================================================
  // Scenario: RoleBanner shows active role text
  // ============================================================================

  describe('Scenario: RoleBanner shows active role text', () => {
    it('should display role text with cyan prefix when role is set', () => {
      // @step Given a session with role set to "security reviewer"
      const roleText = 'security reviewer';

      // @step When the AgentView renders
      const { lastFrame } = render(
        <RoleBanner roleText={roleText} />
      );

      // @step Then a RoleBanner is displayed below the SessionHeader border
      const output = lastFrame();
      expect(output).toBeDefined();

      // @step And the banner shows "Role:" prefix in cyan
      expect(output).toContain('Role:');

      // @step And the banner shows "security reviewer" as dimmed text
      expect(output).toContain('security reviewer');
    });
  });

  // ============================================================================
  // Scenario: RoleBanner hidden when no role set
  // ============================================================================

  describe('Scenario: RoleBanner hidden when no role set', () => {
    it('should render nothing when roleText is null', () => {
      // @step Given a session with no role set
      const roleText = null;

      // @step When the AgentView renders
      const { lastFrame } = render(
        <RoleBanner roleText={roleText} />
      );

      // @step Then no RoleBanner is displayed
      // @step And there is no empty gap between SessionHeader and conversation
      const output = lastFrame();
      expect(output).toBe('');
    });

    it('should render nothing when roleText is empty string', () => {
      // @step Given a session with no role set
      const roleText = '';

      // @step When the AgentView renders
      const { lastFrame } = render(
        <RoleBanner roleText={roleText} />
      );

      // @step Then no RoleBanner is displayed
      // @step And there is no empty gap between SessionHeader and conversation
      const output = lastFrame();
      expect(output).toBe('');
    });
  });

  // ============================================================================
  // Scenario: RoleBanner appears after setting role via /role dialog
  // ============================================================================

  describe('Scenario: RoleBanner appears after setting role via /role dialog', () => {
    it('should display new role text after prop change', () => {
      // @step Given a session with no role set
      const { lastFrame, rerender } = render(
        <RoleBanner roleText={null} />
      );
      expect(lastFrame()).toBe('');

      // @step When the user submits "code reviewer" via the /role dialog
      rerender(<RoleBanner roleText="code reviewer" />);

      // @step Then a RoleBanner appears showing "Role: code reviewer"
      const output = lastFrame();
      expect(output).toContain('Role:');
      expect(output).toContain('code reviewer');
    });
  });

  // ============================================================================
  // Scenario: Long role text is truncated
  // ============================================================================

  describe('Scenario: Long role text is truncated', () => {
    it('should handle very long role text without crashing', () => {
      // @step Given a session with a very long role text
      const longRole = 'You are an expert security reviewer specializing in OWASP Top 10 vulnerabilities with deep knowledge of SQL injection, XSS, CSRF, authentication bypass, and cryptographic weaknesses across multiple programming languages and frameworks';

      // @step When the AgentView renders
      const { lastFrame } = render(
        <RoleBanner roleText={longRole} />
      );

      // @step Then the RoleBanner displays the role text truncated to fit the terminal width
      const output = lastFrame();
      expect(output).toBeDefined();
      expect(output).toContain('Role:');
      // The text should contain at least the start of the role
      expect(output).toContain('You are an expert');
    });
  });
});
