/**
 * Perplexity Research Tool
 *
 * Web research using Perplexity AI during Example Mapping.
 * Bundled TypeScript implementation.
 */

import type { ResearchTool } from './types';
import https from 'https';
import fs from 'fs';
import path from 'path';
import os from 'os';

interface PerplexityConfig {
  apiKey: string;
}

/**
 * Load Perplexity configuration from ~/.fspec/fspec-config.json
 */
function loadConfig(): PerplexityConfig {
  const configPath = path.join(os.homedir(), '.fspec', 'fspec-config.json');

  if (!fs.existsSync(configPath)) {
    throw new Error(
      'Config file not found at ~/.fspec/fspec-config.json\n' +
        'Create config with Perplexity API key:\n' +
        '  mkdir -p ~/.fspec\n' +
        '  echo \'{"research":{"perplexity":{"apiKey":"pplx-..."}}}\' > ~/.fspec/fspec-config.json'
    );
  }

  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

  if (!config.research?.perplexity?.apiKey) {
    throw new Error(
      'Perplexity API key not configured\n' +
        'Add to ~/.fspec/fspec-config.json:\n' +
        '  "research": { "perplexity": { "apiKey": "pplx-..." } }'
    );
  }

  return config.research.perplexity;
}

/**
 * Call Perplexity API
 */
async function callPerplexityAPI(
  query: string,
  config: PerplexityConfig,
  model: string
): Promise<any> {
  return new Promise((resolve, reject) => {
    const payload = JSON.stringify({
      model: model,
      messages: [
        { role: 'system', content: 'Be precise and concise.' },
        { role: 'user', content: query },
      ],
    });

    const options = {
      hostname: 'api.perplexity.ai',
      port: 443,
      path: '/chat/completions',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${config.apiKey}`,
        'Content-Length': Buffer.byteLength(payload),
      },
    };

    const req = https.request(options, res => {
      let data = '';

      res.on('data', chunk => {
        data += chunk;
      });

      res.on('end', () => {
        if (res.statusCode !== 200) {
          const error = JSON.parse(data);
          reject(
            new Error(
              `Perplexity API request failed (HTTP ${res.statusCode})\n` +
                `Reason: ${error.error?.message || 'Unknown error'}\n\n` +
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
            `Fix: Check internet connection and API endpoint availability`
        )
      );
    });

    req.write(payload);
    req.end();
  });
}

/**
 * Format output as markdown
 */
function formatMarkdown(query: string, response: any, model: string): string {
  const answer = response.choices[0].message.content;
  const timestamp = new Date().toISOString();
  const usage = response.usage;

  return `# Research Results: ${query}

**Source:** Perplexity AI (${model})
**Date:** ${timestamp}

---

## Answer

${answer}

---

**Tokens Used:** ${usage.total_tokens} (prompt: ${usage.prompt_tokens}, completion: ${usage.completion_tokens})`;
}

/**
 * Format output as JSON
 */
function formatJSON(query: string, response: any, model: string): string {
  const answer = response.choices[0].message.content;
  const timestamp = new Date().toISOString();
  const usage = response.usage;

  return JSON.stringify(
    {
      query: query,
      source: 'Perplexity AI',
      model: model,
      timestamp: timestamp,
      answer: answer,
      usage: {
        promptTokens: usage.prompt_tokens,
        completionTokens: usage.completion_tokens,
        totalTokens: usage.total_tokens,
      },
    },
    null,
    2
  );
}

/**
 * Format output as plain text
 */
function formatText(response: any): string {
  return response.choices[0].message.content;
}

export const tool: ResearchTool = {
  name: 'perplexity',
  description:
    'Perplexity AI research tool for web search and AI-powered answers',

  async execute(args: string[]): Promise<string> {
    // Parse arguments
    const queryIndex = args.indexOf('--query');
    const modelIndex = args.indexOf('--model');
    const formatIndex = args.indexOf('--format');

    if (queryIndex === -1) {
      throw new Error('Missing required flag: --query');
    }

    const query = args[queryIndex + 1];
    const model = modelIndex >= 0 ? args[modelIndex + 1] : 'sonar';
    const format = formatIndex >= 0 ? args[formatIndex + 1] : 'markdown';

    if (!query) {
      throw new Error('--query value cannot be empty');
    }

    // Load configuration
    const config = loadConfig();

    // Call Perplexity API
    const response = await callPerplexityAPI(query, config, model);

    // Format output
    switch (format) {
      case 'json':
        return formatJSON(query, response, model);
      case 'text':
        return formatText(response);
      case 'markdown':
      default:
        return formatMarkdown(query, response, model);
    }
  },

  help(): string {
    return `PERPLEXITY RESEARCH TOOL

Research questions using Perplexity AI during Example Mapping.

USAGE
  perplexity --query "your question here" [options]

OPTIONS
  --query <text>      Question to research (required)
  --model <name>      Perplexity model (default: sonar)
  --format <type>     Output format: markdown, json, text (default: markdown)
  --help              Show this help message

EXAMPLES
  perplexity --query "How does OAuth2 work?"
  perplexity --query "What is Example Mapping?" --format json

CONFIGURATION
  API key must be set in ~/.fspec/fspec-config.json:
  {
    "research": {
      "perplexity": {
        "apiKey": "pplx-..."
      }
    }
  }

EXIT CODES
  0  Success
  1  Missing required flag (--query)
  2  API key not configured
  3  API error (network, rate limit, etc.)`;
  },
};

// Default export for compatibility
export default tool;
