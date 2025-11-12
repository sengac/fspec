/**
 * Confluence Research Tool
 *
 * Research Confluence pages during Example Mapping.
 * Bundled TypeScript implementation.
 */

import type { ResearchTool } from './types';
import https from 'https';
import { URL } from 'url';
import fs from 'fs';
import path from 'path';
import os from 'os';

interface ConfluenceConfig {
  confluenceUrl: string;
  username: string;
  apiToken: string;
}

/**
 * Load Confluence configuration from ~/.fspec/fspec-config.json
 */
function loadConfig(): ConfluenceConfig {
  const configPath = path.join(os.homedir(), '.fspec', 'fspec-config.json');

  if (!fs.existsSync(configPath)) {
    throw new Error(
      'Config file not found at ~/.fspec/fspec-config.json\n' +
        'Create config with Confluence credentials:\n' +
        '  mkdir -p ~/.fspec\n' +
        '  echo \'{"research":{"confluence":{"confluenceUrl":"https://example.atlassian.net/wiki","username":"your-email","apiToken":"your-token"}}}\' > ~/.fspec/fspec-config.json'
    );
  }

  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

  if (
    !config.research?.confluence?.confluenceUrl ||
    !config.research?.confluence?.username ||
    !config.research?.confluence?.apiToken
  ) {
    throw new Error(
      'Confluence configuration not found\n' +
        'Add to ~/.fspec/fspec-config.json:\n' +
        '  "research": { "confluence": { "confluenceUrl": "...", "username": "...", "apiToken": "..." } }\n\n' +
        'Required config fields: confluenceUrl, username, apiToken'
    );
  }

  return config.research.confluence;
}

/**
 * Call Confluence API
 */
async function callConfluenceAPI(
  config: ConfluenceConfig,
  apiPath: string
): Promise<any> {
  return new Promise((resolve, reject) => {
    const urlObj = new URL(config.confluenceUrl);
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
              `Confluence API authentication failed (HTTP 401)\n` +
                `Reason: Unauthorized - Invalid credentials\n\n` +
                `Fix: Check username and API token in ~/.fspec/fspec-config.json`
            )
          );
        } else if (res.statusCode === 404) {
          reject(
            new Error(
              `Page not found\n\n` + `Fix: Check page title or space key`
            )
          );
        } else if (res.statusCode !== 200) {
          const error = JSON.parse(data);
          reject(
            new Error(
              `Confluence API request failed (HTTP ${res.statusCode})\n` +
                `Reason: ${error.message || 'Unknown error'}\n\n` +
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
            `Fix: Check internet connection and Confluence URL`
        )
      );
    });

    req.end();
  });
}

/**
 * Format single page as markdown
 */
function formatPageMarkdown(page: any): string {
  const timestamp = new Date().toISOString();

  return `# Page: ${page.title}

**Space:** ${page.space?.key || 'Unknown'}
**Modified:** ${page.version?.when || 'Unknown'}
**URL:** ${page._links?.webui ? `${page._links.base}${page._links.webui}` : 'Unknown'}
**Date:** ${timestamp}

---

## Content

${page.body?.storage?.value || page.body?.view?.value || 'No content available'}

---`;
}

/**
 * Format page list as markdown
 */
function formatPagesMarkdown(pages: any[], query: string): string {
  const timestamp = new Date().toISOString();

  let output = `# Confluence Search Results\n\n**Query:** ${query}\n**Date:** ${timestamp}\n**Results:** ${pages.length}\n\n---\n\n`;

  for (const page of pages) {
    output += `## ${page.title}\n\n`;
    output += `**Space:** ${page.space?.key || 'Unknown'}\n`;
    output += `**Excerpt:** ${page.excerpt || 'No excerpt available'}\n`;
    output += `**URL:** ${page._links?.webui ? `${page._links.base}${page._links.webui}` : 'Unknown'}\n\n`;
  }

  return output;
}

/**
 * Format as JSON
 */
function formatJSON(data: any): string {
  if (Array.isArray(data)) {
    return JSON.stringify(
      data.map(page => ({
        id: page.id,
        title: page.title,
        excerpt: page.excerpt || '',
        url: page._links?.webui
          ? `${page._links.base}${page._links.webui}`
          : '',
      })),
      null,
      2
    );
  } else {
    return JSON.stringify(
      {
        id: data.id,
        title: data.title,
        space: data.space?.key,
        excerpt: data.excerpt || '',
        url: data._links?.webui
          ? `${data._links.base}${data._links.webui}`
          : '',
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
      .map(page => `${page.title}: ${page.excerpt || 'No excerpt'}`)
      .join('\n');
  } else {
    return `${data.title}\n\n${data.body?.storage?.value || data.body?.view?.value || 'No content'}`;
  }
}

export const tool: ResearchTool = {
  name: 'confluence',
  description: 'Confluence research tool for searching pages and spaces',

  async execute(args: string[]): Promise<string> {
    // Parse arguments
    const queryIndex = args.indexOf('--query');
    const spaceIndex = args.indexOf('--space');
    const pageIndex = args.indexOf('--page');
    const formatIndex = args.indexOf('--format');

    if (queryIndex === -1 && spaceIndex === -1 && pageIndex === -1) {
      throw new Error(
        'At least one of --query, --space, or --page is required'
      );
    }

    const query = queryIndex >= 0 ? args[queryIndex + 1] : null;
    const space = spaceIndex >= 0 ? args[spaceIndex + 1] : null;
    const page = pageIndex >= 0 ? args[pageIndex + 1] : null;
    const format = formatIndex >= 0 ? args[formatIndex + 1] : 'markdown';

    // Load configuration
    const config = loadConfig();

    // Fetch data from Confluence
    let data;
    let searchQuery = '';

    if (page) {
      // Fetch single page by title
      searchQuery = `Page "${page}"`;
      const result = await callConfluenceAPI(
        config,
        `/wiki/rest/api/content?title=${encodeURIComponent(page)}&expand=body.storage,body.view,space,version`
      );
      if (result.results.length === 0) {
        throw new Error('Page not found');
      }
      data = result.results[0];
    } else if (space) {
      // Fetch pages in space
      searchQuery = `Space ${space}`;
      const result = await callConfluenceAPI(
        config,
        `/wiki/rest/api/content?spaceKey=${space}&expand=space`
      );
      data = result.results;
    } else if (query) {
      // Search with CQL
      searchQuery = query;
      const result = await callConfluenceAPI(
        config,
        `/wiki/rest/api/content/search?cql=${encodeURIComponent(query)}&expand=space`
      );
      data = result.results;
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
          return formatPagesMarkdown(data, searchQuery);
        } else {
          return formatPageMarkdown(data);
        }
    }
  },

  getHelpConfig() {
    return {
      name: 'confluence',
      description: 'Research Confluence pages during Example Mapping',
      usage: 'fspec research --tool=confluence [options]',
      whenToUse:
        'Use during Example Mapping to research documentation, requirements, or design decisions stored in Confluence.',
      options: [
        {
          flag: '--query <text>',
          description: 'Full-text search (required if no --space or --page)',
        },
        {
          flag: '--space <key>',
          description:
            'List all pages in space (required if no --query or --page)',
        },
        {
          flag: '--page <title>',
          description:
            'Fetch single page by title (required if no --query or --space)',
        },
        {
          flag: '--format <type>',
          description: 'Output format',
          defaultValue: 'markdown',
        },
      ],
      examples: [
        {
          command: '--query "API documentation"',
          description: 'Full-text search',
        },
        { command: '--space DOCS', description: 'List pages in space' },
        {
          command: '--page "Authentication Guide"',
          description: 'Fetch specific page',
        },
      ],
      configuration: {
        required: true,
        location: '~/.fspec/fspec-config.json',
        example: JSON.stringify(
          {
            research: {
              confluence: {
                confluenceUrl: 'https://example.atlassian.net/wiki',
                username: 'your-email',
                apiToken: 'your-token',
              },
            },
          },
          null,
          2
        ),
      },
      exitCodes: [
        { code: 0, description: 'Success' },
        { code: 1, description: 'Missing required flag' },
        { code: 2, description: 'Configuration or authentication error' },
        { code: 3, description: 'API error' },
      ],
    };
  },
};

// Default export for compatibility
export default tool;
