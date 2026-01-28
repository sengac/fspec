/**
 * Feature: spec/features/jira-research-tool-outputs-object-object-in-markdown-format-for-description-field.feature
 *
 * Tests that JIRA research tool parses Atlassian Document Format (ADF) descriptions correctly in markdown output.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tool as jiraTool } from '../jira';
import https from 'https';

// Use vi.hoisted to define mocks that will be available during module mocking
const { mockHomedir, mockExistsSync, mockReadFileSync, mockHttpsRequest } =
  vi.hoisted(() => ({
    mockHomedir: vi.fn(),
    mockExistsSync: vi.fn(),
    mockReadFileSync: vi.fn(),
    mockHttpsRequest: vi.fn(),
  }));

// Mock https module
vi.mock('https', async importOriginal => {
  const actual = await importOriginal();
  return {
    ...actual,
    request: mockHttpsRequest,
    default: {
      request: mockHttpsRequest,
    },
  };
});
vi.mock('fs', async importOriginal => {
  const actual = await importOriginal();
  return {
    ...actual,
    existsSync: mockExistsSync,
    readFileSync: mockReadFileSync,
    default: {
      existsSync: mockExistsSync,
      readFileSync: mockReadFileSync,
    },
  };
});
vi.mock('os', async importOriginal => {
  const actual = await importOriginal();
  return {
    ...actual,
    homedir: mockHomedir,
    default: {
      homedir: mockHomedir,
    },
  };
});

describe('Feature: JIRA research tool outputs [object Object] in markdown format for description field', () => {
  beforeEach(async () => {
    vi.resetAllMocks();

    // Mock config file existence and content
    mockHomedir.mockReturnValue('/mock/home');
    mockExistsSync.mockReturnValue(true);
    mockReadFileSync.mockReturnValue(
      JSON.stringify({
        research: {
          jira: {
            jiraUrl: 'https://test.atlassian.net',
            username: 'test@example.com',
            apiToken: 'test-token',
          },
        },
      })
    );
  });

  describe('Scenario: Parse ADF description and display as markdown text', () => {
    it('should parse ADF description and not show [object Object]', async () => {
      // @step Given a JIRA issue has a description in Atlassian Document Format
      const mockAdfDescription = {
        type: 'doc',
        version: 1,
        content: [
          {
            type: 'paragraph',
            content: [
              {
                type: 'text',
                text: 'The frontend interface should be user-friendly and responsive',
              },
            ],
          },
        ],
      };

      const mockIssueData = {
        key: 'CCS-6',
        self: 'https://test.atlassian.net/rest/api/3/issue/10005',
        fields: {
          summary: 'Develop Frontend Interface',
          status: { name: 'To Do' },
          assignee: null,
          labels: [],
          description: mockAdfDescription,
        },
      };

      // Mock https.request to return the mock issue data
      const mockRequest = {
        on: vi.fn(),
        end: vi.fn(),
      };

      const mockResponse = {
        statusCode: 200,
        on: vi.fn((event, callback) => {
          if (event === 'data') {
            callback(JSON.stringify(mockIssueData));
          } else if (event === 'end') {
            callback();
          }
        }),
      };

      mockHttpsRequest.mockImplementation((_options, callback) => {
        callback(mockResponse as any);
        return mockRequest as any;
      });

      // @step When I run "fspec research --tool=jira --issue CCS-6"
      const result = await jiraTool.execute(['--issue', 'CCS-6']);

      // @step Then the output should contain the description text
      expect(result).toContain(
        'The frontend interface should be user-friendly and responsive'
      );

      // @step And the output should not contain "[object Object]"
      expect(result).not.toContain('[object Object]');
    });
  });

  describe('Scenario: Display specific issue description correctly', () => {
    it('should display the actual description text in human-readable format', async () => {
      // @step Given JIRA issue CCS-6 has description "The frontend interface should be user-friendly and responsive"
      const mockAdfDescription = {
        type: 'doc',
        version: 1,
        content: [
          {
            type: 'paragraph',
            content: [
              {
                type: 'text',
                text: 'The frontend interface should be user-friendly and responsive',
              },
            ],
          },
        ],
      };

      const mockIssueData = {
        key: 'CCS-6',
        self: 'https://test.atlassian.net/rest/api/3/issue/10005',
        fields: {
          summary: 'Develop Frontend Interface',
          status: { name: 'To Do' },
          assignee: null,
          labels: [],
          description: mockAdfDescription,
        },
      };

      const mockRequest = {
        on: vi.fn(),
        end: vi.fn(),
      };

      const mockResponse = {
        statusCode: 200,
        on: vi.fn((event, callback) => {
          if (event === 'data') {
            callback(JSON.stringify(mockIssueData));
          } else if (event === 'end') {
            callback();
          }
        }),
      };

      mockHttpsRequest.mockImplementation((_options, callback) => {
        callback(mockResponse as any);
        return mockRequest as any;
      });

      // @step When I run "fspec research --tool=jira --issue CCS-6"
      const result = await jiraTool.execute(['--issue', 'CCS-6']);

      // @step Then the markdown output should contain "The frontend interface should be user-friendly and responsive"
      expect(result).toContain(
        'The frontend interface should be user-friendly and responsive'
      );

      // @step And the description section should be human-readable
      expect(result).toMatch(/## Description[\s\S]*The frontend interface/);
    });
  });

  describe('Scenario: Handle empty or missing description gracefully', () => {
    it('should show "No description provided" instead of [object Object]', async () => {
      // @step Given a JIRA issue has no description field
      const mockIssueData = {
        key: 'CCS-7',
        self: 'https://test.atlassian.net/rest/api/3/issue/10006',
        fields: {
          summary: 'Issue Without Description',
          status: { name: 'To Do' },
          assignee: null,
          labels: [],
          description: null,
        },
      };

      const mockRequest = {
        on: vi.fn(),
        end: vi.fn(),
      };

      const mockResponse = {
        statusCode: 200,
        on: vi.fn((event, callback) => {
          if (event === 'data') {
            callback(JSON.stringify(mockIssueData));
          } else if (event === 'end') {
            callback();
          }
        }),
      };

      mockHttpsRequest.mockImplementation((_options, callback) => {
        callback(mockResponse as any);
        return mockRequest as any;
      });

      // @step When I run "fspec research --tool=jira --issue" with that issue key
      const result = await jiraTool.execute(['--issue', 'CCS-7']);

      // @step Then the output should contain "No description provided"
      expect(result).toContain('No description provided');

      // @step And the output should not contain "[object Object]"
      expect(result).not.toContain('[object Object]');
    });
  });
});
