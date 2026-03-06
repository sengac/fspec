/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the WebMCP tool naming utilities (EXT-006),
 * specifically the origin sanitization that ensures tool names comply
 * with the Anthropic API pattern: ^[a-zA-Z0-9_-]{1,128}$
 *
 * Dots in hostnames (e.g., "app.example.com") must be replaced with
 * hyphens to prevent API 400 errors.
 */

import { describe, it, expect } from 'vitest';
import {
  sanitizeOrigin,
  buildWebmcpToolName,
  parseWebmcpToolName,
} from '../webmcp-naming';

/** Anthropic API tool name pattern */
const API_TOOL_NAME_PATTERN = /^[a-zA-Z0-9_-]{1,128}$/;

describe('Feature: WebMCP Tool Naming - Origin Sanitization', () => {
  describe('sanitizeOrigin', () => {
    it('should replace dots with hyphens in standard hostnames', () => {
      expect(sanitizeOrigin('app.example.com')).toBe('app-example-com');
    });

    it('should replace dots in simple domains', () => {
      expect(sanitizeOrigin('example.com')).toBe('example-com');
    });

    it('should replace colons in localhost with port', () => {
      expect(sanitizeOrigin('localhost:3000')).toBe('localhost-3000');
    });

    it('should leave already-safe strings unchanged', () => {
      expect(sanitizeOrigin('localhost')).toBe('localhost');
      expect(sanitizeOrigin('my-server')).toBe('my-server');
      expect(sanitizeOrigin('my_server')).toBe('my_server');
    });

    it('should handle multiple consecutive dots', () => {
      expect(sanitizeOrigin('a..b')).toBe('a--b');
    });

    it('should handle subdomains with many segments', () => {
      expect(sanitizeOrigin('a.b.c.d.example.com')).toBe('a-b-c-d-example-com');
    });

    it('should replace all non-alphanumeric/non-underscore/non-hyphen characters', () => {
      expect(sanitizeOrigin('host:8080/path?q=1')).toBe('host-8080-path-q-1');
    });

    it('should handle IP addresses', () => {
      expect(sanitizeOrigin('192.168.1.1')).toBe('192-168-1-1');
    });

    it('should handle IP addresses with port', () => {
      expect(sanitizeOrigin('192.168.1.1:8080')).toBe('192-168-1-1-8080');
    });

    it('should handle empty string', () => {
      expect(sanitizeOrigin('')).toBe('');
    });
  });

  describe('buildWebmcpToolName', () => {
    it('should produce API-compliant names for dotted hostnames', () => {
      const name = buildWebmcpToolName('app.example.com', 'getApiRequests');
      expect(name).toBe('webmcp__app-example-com__getApiRequests');
      // Every segment between __ separators must be API-compliant
      const segments = name.split('__');
      for (const segment of segments) {
        expect(segment).toMatch(API_TOOL_NAME_PATTERN);
      }
    });

    it('should produce API-compliant names for simple domains', () => {
      const name = buildWebmcpToolName('example.com', 'searchFlights');
      expect(name).toBe('webmcp__example-com__searchFlights');
    });

    it('should produce API-compliant names for localhost with port', () => {
      const name = buildWebmcpToolName('localhost:3000', 'getData');
      expect(name).toBe('webmcp__localhost-3000__getData');
    });

    it('should leave already-safe origins unchanged', () => {
      const name = buildWebmcpToolName('localhost', 'ping');
      expect(name).toBe('webmcp__localhost__ping');
    });

    it('should produce a fully API-compliant qualified name when prefixed with mcp__', () => {
      // This simulates what codelet does: mcp__<server>__<tool>
      // where <tool> is the full webmcp__<origin>__<toolName> string
      const webmcpName = buildWebmcpToolName(
        'app.example.com',
        'getApiRequests'
      );
      const qualifiedName = `mcp__webmcp__${webmcpName.slice('webmcp__'.length)}`;
      // The full name should not contain dots
      expect(qualifiedName).not.toContain('.');
    });
  });

  describe('parseWebmcpToolName', () => {
    it('should parse sanitized origin names correctly', () => {
      const result = parseWebmcpToolName(
        'webmcp__app-example-com__getApiRequests'
      );
      expect(result).toEqual({
        origin: 'app-example-com',
        toolName: 'getApiRequests',
      });
    });

    it('should parse simple domain names', () => {
      const result = parseWebmcpToolName('webmcp__example-com__searchFlights');
      expect(result).toEqual({
        origin: 'example-com',
        toolName: 'searchFlights',
      });
    });

    it('should parse localhost names', () => {
      const result = parseWebmcpToolName('webmcp__localhost__ping');
      expect(result).toEqual({ origin: 'localhost', toolName: 'ping' });
    });

    it('should parse localhost with sanitized port', () => {
      const result = parseWebmcpToolName('webmcp__localhost-3000__getData');
      expect(result).toEqual({ origin: 'localhost-3000', toolName: 'getData' });
    });

    it('should return undefined for names without webmcp prefix', () => {
      expect(parseWebmcpToolName('other__origin__tool')).toBeUndefined();
    });

    it('should return undefined for names without separators', () => {
      expect(parseWebmcpToolName('webmcptool')).toBeUndefined();
    });

    it('should return undefined for empty origin', () => {
      expect(parseWebmcpToolName('webmcp____tool')).toBeUndefined();
    });

    it('should return undefined for empty tool name', () => {
      expect(parseWebmcpToolName('webmcp__origin__')).toBeUndefined();
    });

    it('should handle tool names containing double underscores', () => {
      const result = parseWebmcpToolName('webmcp__example-com__tool__subtool');
      expect(result).toEqual({
        origin: 'example-com',
        toolName: 'tool__subtool',
      });
    });
  });

  describe('buildWebmcpToolName → parseWebmcpToolName roundtrip', () => {
    it('should roundtrip with dotted hostname', () => {
      const built = buildWebmcpToolName('app.example.com', 'getApiRequests');
      const parsed = parseWebmcpToolName(built);
      expect(parsed).toBeDefined();
      expect(parsed?.origin).toBe('app-example-com');
      expect(parsed?.toolName).toBe('getApiRequests');
    });

    it('should roundtrip with simple hostname', () => {
      const built = buildWebmcpToolName('localhost', 'ping');
      const parsed = parseWebmcpToolName(built);
      expect(parsed).toBeDefined();
      expect(parsed?.origin).toBe('localhost');
      expect(parsed?.toolName).toBe('ping');
    });

    it('should roundtrip with port in hostname', () => {
      const built = buildWebmcpToolName('localhost:8080', 'status');
      const parsed = parseWebmcpToolName(built);
      expect(parsed).toBeDefined();
      expect(parsed?.origin).toBe('localhost-8080');
      expect(parsed?.toolName).toBe('status');
    });
  });

  describe('API compliance: all built names must match Anthropic pattern', () => {
    const testCases = [
      { origin: 'app.example.com', tool: 'getApiRequests' },
      { origin: 'example.com', tool: 'searchFlights' },
      { origin: 'sub.domain.example.co.uk', tool: 'doThing' },
      { origin: 'localhost:3000', tool: 'getData' },
      { origin: '192.168.1.1', tool: 'ping' },
      { origin: '192.168.1.1:8080', tool: 'status' },
      { origin: 'my-server', tool: 'check' },
      { origin: 'my_server', tool: 'check' },
    ];

    for (const { origin, tool } of testCases) {
      it(`should produce API-compliant name for origin "${origin}"`, () => {
        const name = buildWebmcpToolName(origin, tool);
        // The full name (as it would appear when qualified by codelet)
        // is mcp__webmcp__<sanitized-origin>__<tool>
        // But the MCP server exposes just the webmcp__... part.
        // Each segment between __ separators must be API-compliant.
        const segments = name.split('__');
        for (const segment of segments) {
          expect(segment).toMatch(API_TOOL_NAME_PATTERN);
        }
        // The full name must not contain dots
        expect(name).not.toContain('.');
      });
    }
  });
});
