#!/usr/bin/env node
/**
 * Hook: Send Slack notification about work unit status change
 *
 * This hook reads the context from stdin, extracts the workUnitId and event,
 * and sends a notification to a Slack webhook.
 */

const https = require('https');
const { stdin } = require('process');

// Read JSON context from stdin
let contextData = '';

stdin.on('data', (chunk) => {
  contextData += chunk;
});

stdin.on('end', async () => {
  try {
    // Parse JSON context
    const context = JSON.parse(contextData);

    // Extract workUnitId and event from context
    const { workUnitId, event, timestamp } = context;

    if (!workUnitId || !event) {
      console.error('Missing workUnitId or event in context');
      process.exit(1);
    }

    // Slack webhook URL (from environment variable)
    const webhookUrl = process.env.SLACK_WEBHOOK_URL;

    if (!webhookUrl) {
      console.error('SLACK_WEBHOOK_URL environment variable not set');
      process.exit(1);
    }

    // Create message
    const message = {
      text: `Work unit ${workUnitId} - ${event}`,
      blocks: [
        {
          type: 'section',
          text: {
            type: 'mrkdwn',
            text: `*${workUnitId}* triggered event: \`${event}\``,
          },
        },
        {
          type: 'context',
          elements: [
            {
              type: 'mrkdwn',
              text: `Timestamp: ${timestamp}`,
            },
          ],
        },
      ],
    };

    // Send HTTP request to Slack webhook
    const url = new URL(webhookUrl);
    const postData = JSON.stringify(message);

    const options = {
      hostname: url.hostname,
      port: 443,
      path: url.pathname,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(postData),
      },
    };

    const req = https.request(options, (res) => {
      let responseData = '';

      res.on('data', (chunk) => {
        responseData += chunk;
      });

      res.on('end', () => {
        if (res.statusCode === 200) {
          console.log(`✓ Notification sent for ${workUnitId}`);
          process.exit(0);
        } else {
          console.error(`✗ Slack webhook returned ${res.statusCode}`);
          process.exit(1);
        }
      });
    });

    // Proper error handling for network failures
    req.on('error', (error) => {
      console.error(`✗ Failed to send notification: ${error.message}`);
      process.exit(1);
    });

    req.write(postData);
    req.end();
  } catch (error) {
    console.error(`Failed to parse context or send notification: ${error.message}`);
    process.exit(1);
  }
});
