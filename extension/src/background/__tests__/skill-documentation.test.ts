/**
 * Feature: spec/features/browser-scan-diff-documentation.feature
 *
 * This test file validates the acceptance criteria for MCP Tool Definitions
 * & Skill Documentation. It verifies that documentation files contain
 * accurate and complete information about the scan, diff, and ref tools.
 */

import { readFileSync } from 'fs';
import { join } from 'path';

const EXTENSION_ROOT = join(__dirname, '..', '..', '..');
const SKILL_FILE = join(EXTENSION_ROOT, 'webmcp-skill.md');
const MCP_SERVER_FILE = join(EXTENSION_ROOT, 'host', 'lib', 'mcp-server.mjs');
const INJECT_SKILL_FILE = join(EXTENSION_ROOT, 'inject-webmcp-tools-skill.md');

describe('Feature: MCP Tool Definitions & Skill Documentation', () => {
  let skillContent: string;
  let mcpServerContent: string;
  let injectSkillContent: string;

  beforeAll(() => {
    skillContent = readFileSync(SKILL_FILE, 'utf-8');
    mcpServerContent = readFileSync(MCP_SERVER_FILE, 'utf-8');
    injectSkillContent = readFileSync(INJECT_SKILL_FILE, 'utf-8');
  });

  describe('Scenario: Skill documentation lists all 14 native tools', () => {
    it('should state 14 native tools and list scan/diff tools', () => {
      // @step Given the webmcp-skill.md file exists
      // (validated by beforeAll — readFileSync throws if missing)

      // @step When I read the header section
      const headerSection = skillContent.split(
        '## Native Browser Control Tools'
      )[0];

      // @step Then it should state 14 native browser control tools
      expect(headerSection).toContain('14 native browser control tools');

      // @step And browser_scan_page and browser_diff_page should be listed in the Native Browser Control Tools section
      const toolsSection =
        skillContent
          .split('## Native Browser Control Tools')[1]
          ?.split('## WebMCP')[0] ?? '';
      expect(toolsSection).toContain('browser_scan_page');
      expect(toolsSection).toContain('browser_diff_page');
    });
  });

  describe('Scenario: Click and fill tools document ref syntax', () => {
    it('should mention @ref syntax in click and fill tool docs', () => {
      // @step Given the webmcp-skill.md file exists
      // (validated by beforeAll — readFileSync throws if missing)

      // @step When I read the browser_click_element and browser_fill_form documentation
      const clickSection =
        skillContent.split('### `browser_click_element`')[1]?.split('###')[0] ??
        '';
      const fillSection =
        skillContent.split('### `browser_fill_form`')[1]?.split('###')[0] ?? '';

      // @step Then the selector parameter should mention accepting @ref syntax from browser_scan_page
      expect(clickSection).toMatch(/@ref|@e\d|ref.*browser_scan_page/i);
      expect(fillSection).toMatch(/@ref|@e\d|ref.*browser_scan_page/i);
    });
  });

  describe('Scenario: Common workflows include scan-interact-verify pattern', () => {
    it('should include the scan-interact-verify workflow as a unified block', () => {
      // @step Given the webmcp-skill.md file exists
      // (validated by beforeAll — readFileSync throws if missing)

      // @step When I read the Common Workflows section
      const workflowSection =
        skillContent.split('## Common Workflows')[1] ?? '';

      // @step Then it should include a workflow showing navigate, scan, fill, click, diff, and re-scan steps
      // Extract the specific "refs" workflow subsection to verify all steps exist together
      const refsWorkflow =
        workflowSection
          .split('### Interact with page elements using refs')[1]
          ?.split('###')[0] ??
        workflowSection
          .split('### Interact with page elements')[1]
          ?.split('###')[0] ??
        '';

      // All 6 required tools must appear in this single workflow block
      expect(refsWorkflow).toContain('browser_navigate');
      expect(refsWorkflow).toContain('browser_scan_page');
      expect(refsWorkflow).toContain('browser_fill_form');
      expect(refsWorkflow).toContain('browser_click_element');
      expect(refsWorkflow).toContain('browser_diff_page');

      // Re-scan: browser_scan_page must appear at least twice (initial scan + re-scan)
      const scanOccurrences = refsWorkflow.match(/browser_scan_page/g) ?? [];
      expect(scanOccurrences.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Scenario: Ref lifecycle documentation', () => {
    it('should explain ref assignment and ephemeral nature', () => {
      // @step Given the webmcp-skill.md file exists
      // (validated by beforeAll — readFileSync throws if missing)

      // @step When I read the Ref Lifecycle section
      const refSection =
        skillContent.split(/## Ref Lifecycle/i)[1]?.split('##')[0] ?? '';

      // @step Then it should explain that refs are assigned by browser_scan_page and are ephemeral
      expect(refSection).toContain('browser_scan_page');
      expect(refSection).toMatch(/ephemeral/i);

      // @step And it should state that refs are invalidated on page navigation
      expect(refSection).toMatch(/invalidat|navigat/i);
    });
  });

  describe('Scenario: MCP server NATIVE_TOOLS includes scan and diff tools', () => {
    it('should contain browser_scan_page and browser_diff_page with correct schemas', () => {
      // @step Given the mcp-server.mjs file exists
      // (validated by beforeAll — readFileSync throws if missing)

      // @step When I inspect the NATIVE_TOOLS array
      const nativeToolsSection =
        mcpServerContent.split('const NATIVE_TOOLS')[1]?.split('];')[0] ?? '';

      // @step Then it should contain browser_scan_page with tabId, interactive, and selector properties
      expect(nativeToolsSection).toContain("name: 'browser_scan_page'");
      expect(nativeToolsSection).toContain('tabId');
      expect(nativeToolsSection).toContain('interactive');
      expect(nativeToolsSection).toContain('selector');

      // @step And it should contain browser_diff_page with tabId property
      expect(nativeToolsSection).toContain("name: 'browser_diff_page'");

      // Verify click/fill tool descriptions mention @ref so agents discover it via tools/list
      const clickToolBlock =
        nativeToolsSection
          .split("name: 'browser_click_element'")[1]
          ?.split('},')[0] ?? '';
      const fillToolBlock =
        nativeToolsSection
          .split("name: 'browser_fill_form'")[1]
          ?.split('},')[0] ?? '';
      expect(clickToolBlock).toMatch(/@ref|ref/i);
      expect(fillToolBlock).toMatch(/@ref|ref/i);
    });
  });

  describe('Scenario: Troubleshooting covers ref-related errors', () => {
    it('should include guidance for ref errors', () => {
      // @step Given the webmcp-skill.md file exists
      // (validated by beforeAll — readFileSync throws if missing)

      // @step When I read the Troubleshooting section
      const troubleshootSection =
        skillContent.split('## Troubleshooting')[1] ?? '';

      // @step Then it should include guidance for ref not found errors suggesting to re-scan
      expect(troubleshootSection).toMatch(/ref.*not found|stale ref|re-scan/i);
    });
  });

  describe('Scenario: Inject skill file has no stale references', () => {
    it('should not contain old extension name references', () => {
      // @step Given the inject-webmcp-tools-skill.md file exists
      // (validated by beforeAll — readFileSync throws if missing)

      // @step When I search for old extension name references
      const staleNames = ['WebMCP Chrome Extension', 'webmcp-extension'];

      // @step Then no references to old tool names or old extension names should be found
      for (const staleName of staleNames) {
        expect(injectSkillContent).not.toContain(staleName);
      }
      // Should use current naming
      expect(injectSkillContent).toContain('fspec Browser Agent');
    });
  });
});
