/**
 * JIRA Research Tool
 *
 * Research JIRA issues during Example Mapping.
 * Bundled TypeScript implementation.
 */

import type { ResearchTool } from './types';
import https from 'https';
import { URL } from 'url';
import fs from 'fs';
import path from 'path';
import os from 'os';

interface JiraConfig {
  jiraUrl: string;
  username: string;
  apiToken: string;
}

/**
 * Load JIRA configuration from ~/.fspec/fspec-config.json
 */
function loadConfig(): JiraConfig {
  const configPath = path.join(os.homedir(), '.fspec', 'fspec-config.json');

  if (!fs.existsSync(configPath)) {
    throw new Error(
      'Config file not found at ~/.fspec/fspec-config.json\n' +
        'Create config with JIRA credentials:\n' +
        '  mkdir -p ~/.fspec\n' +
        '  echo \'{"research":{"jira":{"jiraUrl":"https://example.atlassian.net","username":"your-email","apiToken":"your-token"}}}\' > ~/.fspec/fspec-config.json'
    );
  }

  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

  if (
    !config.research?.jira?.jiraUrl ||
    !config.research?.jira?.username ||
    !config.research?.jira?.apiToken
  ) {
    throw new Error(
      'JIRA configuration not found\n' +
        'Add to ~/.fspec/fspec-config.json:\n' +
        '  "research": { "jira": { "jiraUrl": "...", "username": "...", "apiToken": "..." } }\n\n' +
        'Required config fields: jiraUrl, username, apiToken'
    );
  }

  return config.research.jira;
}

/**
 * Call JIRA API
 */
async function callJiraAPI(config: JiraConfig, apiPath: string): Promise<any> {
  return new Promise((resolve, reject) => {
    const urlObj = new URL(config.jiraUrl);
    const auth = Buffer.from(`${config.username}:${config.apiToken}`).toString(
      'base64'
    );

    const options = {
      hostname: urlObj.hostname,
      port: 443,
      path: apiPath,
      method: 'GET',
      headers: {
        Authorization: `Basic ${auth}`,
        Accept: 'application/json',
      },
    };

    const req = https.request(options, res => {
      let data = '';

      res.on('data', chunk => {
        data += chunk;
      });

      res.on('end', () => {
        if (res.statusCode === 401) {
          reject(
            new Error(
              `JIRA API authentication failed (HTTP 401)\n` +
                `Reason: Unauthorized - Invalid credentials\n\n` +
                `Fix: Check username and API token in ~/.fspec/fspec-config.json`
            )
          );
        } else if (res.statusCode === 404) {
          reject(
            new Error(
              `JIRA API request failed (HTTP 404)\n` +
                `Reason: Issue not found\n\n` +
                `Fix: Check issue key or JQL query`
            )
          );
        } else if (res.statusCode !== 200) {
          const error = JSON.parse(data);
          reject(
            new Error(
              `JIRA API request failed (HTTP ${res.statusCode})\n` +
                `Reason: ${error.errorMessages?.[0] || 'Unknown error'}\n\n` +
                `API Response:\n${JSON.stringify(error, null, 2)}`
            )
          );
        } else {
          resolve(JSON.parse(data));
        }
      });
    });

    req.on('error', err => {
      reject(
        new Error(
          `Network request failed\n` +
            `Reason: ${err.message}\n\n` +
            `Fix: Check internet connection and JIRA URL`
        )
      );
    });

    req.end();
  });
}

/**
 * Parse Atlassian Document Format (ADF) to plain text
 */
function parseADF(adf: any): string {
  if (!adf || typeof adf !== 'object') {
    return '';
  }

  // If it's already a string, return it
  if (typeof adf === 'string') {
    return adf;
  }

  // If it's an ADF document with content array
  if (adf.type === 'doc' && Array.isArray(adf.content)) {
    return adf.content.map((node: any) => parseADFNode(node)).join('\n\n');
  }

  return '';
}

/**
 * Parse individual ADF node recursively
 */
function parseADFNode(node: any): string {
  if (!node || typeof node !== 'object') {
    return '';
  }

  // Handle text nodes
  if (node.type === 'text') {
    return node.text || '';
  }

  // Handle paragraph nodes
  if (node.type === 'paragraph' && Array.isArray(node.content)) {
    return node.content.map((child: any) => parseADFNode(child)).join('');
  }

  // Handle heading nodes
  if (node.type === 'heading' && Array.isArray(node.content)) {
    const level = node.attrs?.level || 1;
    const text = node.content.map((child: any) => parseADFNode(child)).join('');
    return '#'.repeat(level) + ' ' + text;
  }

  // Handle bullet list
  if (node.type === 'bulletList' && Array.isArray(node.content)) {
    return node.content
      .map((item: any) => '- ' + parseADFNode(item))
      .join('\n');
  }

  // Handle ordered list
  if (node.type === 'orderedList' && Array.isArray(node.content)) {
    return node.content
      .map((item: any, index: number) => `${index + 1}. ${parseADFNode(item)}`)
      .join('\n');
  }

  // Handle list item
  if (node.type === 'listItem' && Array.isArray(node.content)) {
    return node.content.map((child: any) => parseADFNode(child)).join('');
  }

  // Handle code block
  if (node.type === 'codeBlock' && Array.isArray(node.content)) {
    return (
      '```\n' +
      node.content.map((child: any) => parseADFNode(child)).join('') +
      '\n```'
    );
  }

  // Handle other nodes with content
  if (Array.isArray(node.content)) {
    return node.content.map((child: any) => parseADFNode(child)).join('');
  }

  return '';
}

/**
 * Format single issue as markdown
 */
function formatIssueMarkdown(issue: any): string {
  const timestamp = new Date().toISOString();

  // Parse description from ADF format to plain text
  const descriptionText = issue.fields?.description
    ? parseADF(issue.fields.description)
    : 'No description provided';

  return `# Issue: ${issue.key}

**Summary:** ${issue.fields?.summary || 'No summary'}
**Status:** ${issue.fields?.status?.name || 'Unknown'}
**Assignee:** ${issue.fields?.assignee?.displayName || 'Unassigned'}
**Labels:** ${issue.fields?.labels?.join(', ') || 'None'}
**Date:** ${timestamp}

---

## Description

${descriptionText}

---

**URL:** ${issue.self}`;
}

/**
 * Format issue list as markdown
 */
function formatIssuesMarkdown(issues: any[], query: string): string {
  const timestamp = new Date().toISOString();

  let output = `# JIRA Search Results\n\n**Query:** ${query}\n**Date:** ${timestamp}\n**Results:** ${issues.length}\n\n---\n\n`;

  for (const issue of issues) {
    output += `## ${issue.key}: ${issue.fields?.summary || 'No summary'}\n\n`;
    output += `**Status:** ${issue.fields?.status?.name || 'Unknown'}\n`;
    output += `**Assignee:** ${issue.fields?.assignee?.displayName || 'Unassigned'}\n`;
    output += `**URL:** ${issue.self}\n\n`;
  }

  return output;
}

/**
 * Format as JSON
 */
function formatJSON(data: any): string {
  if (Array.isArray(data)) {
    return JSON.stringify(
      data.map(issue => ({
        key: issue.key,
        summary: issue.fields?.summary || 'No summary',
        status: issue.fields?.status?.name || 'Unknown',
        assignee: issue.fields?.assignee?.displayName || 'Unassigned',
        url: issue.self,
      })),
      null,
      2
    );
  } else {
    return JSON.stringify(
      {
        key: data.key,
        summary: data.fields?.summary || 'No summary',
        status: data.fields?.status?.name || 'Unknown',
        assignee: data.fields?.assignee?.displayName || 'Unassigned',
        labels: data.fields?.labels || [],
        description: data.fields?.description || 'No description',
        url: data.self,
      },
      null,
      2
    );
  }
}

/**
 * Format as plain text
 */
function formatText(data: any): string {
  if (Array.isArray(data)) {
    return data
      .map(issue => `${issue.key}: ${issue.fields?.summary || 'No summary'}`)
      .join('\n');
  } else {
    return `${data.key}: ${data.fields?.summary || 'No summary'}\n\n${data.fields?.description || 'No description'}`;
  }
}

export const tool: ResearchTool = {
  name: 'jira',
  description: 'JIRA research tool for fetching issues and running JQL queries',

  async execute(args: string[]): Promise<string> {
    // Parse arguments
    const issueIndex = args.indexOf('--issue');
    const projectIndex = args.indexOf('--project');
    const queryIndex = args.indexOf('--query');
    const formatIndex = args.indexOf('--format');

    if (issueIndex === -1 && projectIndex === -1 && queryIndex === -1) {
      throw new Error(
        'At least one of --query, --issue, or --project is required'
      );
    }

    const issue = issueIndex >= 0 ? args[issueIndex + 1] : null;
    const project = projectIndex >= 0 ? args[projectIndex + 1] : null;
    const query = queryIndex >= 0 ? args[queryIndex + 1] : null;
    const format = formatIndex >= 0 ? args[formatIndex + 1] : 'markdown';

    // Load configuration
    const config = loadConfig();

    // Fetch data from JIRA
    let data;
    let searchQuery = '';

    if (issue) {
      // Fetch single issue
      searchQuery = `Issue ${issue}`;
      data = await callJiraAPI(config, `/rest/api/3/issue/${issue}`);
    } else if (project) {
      // Fetch project issues
      searchQuery = `Project ${project}`;
      const result = await callJiraAPI(
        config,
        `/rest/api/3/search/jql?jql=project=${project}&fields=summary,status,assignee,labels,description&maxResults=100`
      );
      data = result.issues;
    } else if (query) {
      // JQL query
      searchQuery = query;
      const result = await callJiraAPI(
        config,
        `/rest/api/3/search/jql?jql=${encodeURIComponent(query)}&fields=summary,status,assignee,labels,description&maxResults=100`
      );
      data = result.issues;
    }

    // Debug: log the data structure
    if (!data) {
      throw new Error('No data returned from JIRA API');
    }
    if (Array.isArray(data) && data.length === 0) {
      return 'No issues found matching the query.';
    }

    // Format output
    switch (format) {
      case 'json':
        return formatJSON(data);
      case 'text':
        return formatText(data);
      case 'markdown':
      default:
        if (Array.isArray(data)) {
          return formatIssuesMarkdown(data, searchQuery);
        } else {
          return formatIssueMarkdown(data);
        }
    }
  },

  getHelpConfig() {
    return {
      name: 'jira',
      description:
        'JIRA research tool for fetching issues and running JQL queries',
      usage: 'fspec research --tool=jira [options]',
      whenToUse:
        'Use during Example Mapping to research existing JIRA issues, understand requirements from tickets, or verify acceptance criteria.',
      options: [
        {
          flag: '--issue <key>',
          description:
            'Fetch single issue by key (required if no --project or --query)',
        },
        {
          flag: '--project <key>',
          description:
            'List all issues in project (required if no --issue or --query)',
        },
        {
          flag: '--query <jql>',
          description:
            'JQL query to search issues (required if no --issue or --project)',
        },
        {
          flag: '--format <type>',
          description: 'Output format',
          defaultValue: 'markdown',
        },
      ],
      examples: [
        {
          command: '--issue AUTH-001',
          description: 'Fetch single issue by key',
        },
        {
          command: '--project AUTH',
          description: 'List all issues in project',
        },
        {
          command: '--query "project = AUTH AND status = Open"',
          description: 'Search issues with JQL query',
        },
      ],
      configuration: {
        required: true,
        location: '~/.fspec/fspec-config.json',
        example: JSON.stringify(
          {
            research: {
              jira: {
                jiraUrl: 'https://example.atlassian.net',
                username: 'your-email@example.com',
                apiToken: 'your-api-token',
              },
            },
          },
          null,
          2
        ),
        instructions: 'API credentials must be set in config file',
      },
      commonErrors: [
        {
          error: 'Missing required flag (--issue, --project, or --query)',
          fix: 'Provide at least one of --issue, --project, or --query',
        },
        {
          error: 'Configuration or authentication error',
          fix: 'Check ~/.fspec/fspec-config.json has valid JIRA credentials',
        },
      ],
      exitCodes: [
        { code: 0, description: 'Success' },
        { code: 1, description: 'Missing required flag' },
        { code: 2, description: 'Configuration or authentication error' },
        { code: 3, description: 'API error (network, not found, etc.)' },
      ],
    };
  },
};

// Default export for compatibility
export default tool;
