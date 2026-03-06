/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * Tests for the WebMCP tool naming utility (EXT-006).
 * Rule [1]: WebMCP tools MUST be namespaced as webmcp__<origin>__<toolName>
 */

import { describe, it, expect } from 'vitest';
import {
  buildWebmcpToolName,
  parseWebmcpToolName,
} from '../../../extension/src/background/webmcp-naming';

describe('WebMCP Tool Naming Utilities', () => {
  describe('buildWebmcpToolName', () => {
    it('should produce the correct 3-segment format', () => {
      expect(buildWebmcpToolName('example.com', 'submitForm')).toBe(
        'webmcp__example.com__submitForm'
      );
    });

    it('should handle origins with subdomains', () => {
      expect(
        buildWebmcpToolName('travel-demo.bandarra.me', 'searchFlights')
      ).toBe('webmcp__travel-demo.bandarra.me__searchFlights');
    });

    it('should handle tool names with underscores', () => {
      expect(buildWebmcpToolName('example.com', 'my_tool_name')).toBe(
        'webmcp__example.com__my_tool_name'
      );
    });
  });

  describe('parseWebmcpToolName', () => {
    it('should parse a valid namespaced name', () => {
      const result = parseWebmcpToolName('webmcp__example.com__submitForm');
      expect(result).toEqual({ origin: 'example.com', toolName: 'submitForm' });
    });

    it('should parse names with subdomain origins', () => {
      const result = parseWebmcpToolName(
        'webmcp__travel-demo.bandarra.me__searchFlights'
      );
      expect(result).toEqual({
        origin: 'travel-demo.bandarra.me',
        toolName: 'searchFlights',
      });
    });

    it('should handle tool names containing double underscores', () => {
      const result = parseWebmcpToolName('webmcp__example.com__my__tool');
      expect(result).toEqual({ origin: 'example.com', toolName: 'my__tool' });
    });

    it('should return undefined for non-webmcp prefixed names', () => {
      expect(parseWebmcpToolName('native__browser__navigate')).toBeUndefined();
    });

    it('should return undefined for names without origin segment', () => {
      expect(parseWebmcpToolName('webmcp__toolOnly')).toBeUndefined();
    });

    it('should return undefined for names with no separators', () => {
      expect(parseWebmcpToolName('plainToolName')).toBeUndefined();
    });

    it('should return undefined for empty origin', () => {
      expect(parseWebmcpToolName('webmcp____toolName')).toBeUndefined();
    });

    it('should return undefined for empty tool name', () => {
      expect(parseWebmcpToolName('webmcp__example.com__')).toBeUndefined();
    });
  });

  describe('roundtrip', () => {
    it('should produce the same name after build then parse', () => {
      const name = buildWebmcpToolName('example.com', 'submitForm');
      const parsed = parseWebmcpToolName(name);
      expect(parsed).toEqual({ origin: 'example.com', toolName: 'submitForm' });
    });
  });
});
