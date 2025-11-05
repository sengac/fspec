/**
 * Attachment Server - Local HTTP server for rendering markdown attachments
 *
 * Serves markdown files with mermaid diagram support for TUI-019 attachment viewing.
 * Server lifecycle is tied to TUI component lifecycle (only runs when TUI is active).
 *
 * Coverage:
 * - TUI-020: Local web server for rendering markdown/mermaid attachments
 */

import express, { type Express, type Request, type Response } from 'express';
import { readFile } from 'fs/promises';
import path from 'path';
import { logger } from '../utils/logger.js';
import { renderMarkdown } from './utils/markdown-renderer.js';
import { getViewerTemplate } from './templates/viewer-template.js';
import type { Server } from 'http';

export interface AttachmentServerOptions {
  port?: number;
  cwd: string; // Base directory for resolving attachment paths
}

/**
 * Creates and starts the attachment server.
 *
 * @param options - Server configuration options
 * @returns Promise resolving to HTTP Server instance
 */
export async function startAttachmentServer(
  options: AttachmentServerOptions
): Promise<Server> {
  const { port = 0, cwd } = options; // port = 0 means random available port
  const app: Express = express();

  // Security: Validate and resolve file paths to prevent directory traversal
  const validatePath = (filePath: string): string | null => {
    try {
      // Resolve relative path against cwd
      const absolutePath = path.isAbsolute(filePath)
        ? filePath
        : path.resolve(cwd, filePath);

      // Normalize to remove .. and .
      const normalizedPath = path.normalize(absolutePath);

      // Ensure the resolved path is within cwd (prevent directory traversal)
      if (!normalizedPath.startsWith(path.normalize(cwd))) {
        logger.warn(
          `[AttachmentServer] Path traversal attempt blocked: ${filePath}`
        );
        return null;
      }

      return normalizedPath;
    } catch (error) {
      logger.error(`[AttachmentServer] Path validation error: ${error}`);
      return null;
    }
  };

  // Route: Serve rendered markdown files
  // Example: http://localhost:PORT/view/spec/attachments/TUI-012/diagram.md
  app.get('/view/*filepath', async (req: Request, res: Response) => {
    try {
      // Extract file path from URL (everything after /view/)
      // In Express 5, *filepath creates req.params.filepath as an ARRAY of path segments
      logger.info(`[AttachmentServer] Raw params:`, req.params);

      const filepathParam = req.params.filepath;
      logger.info(
        `[AttachmentServer] Filepath param type: ${Array.isArray(filepathParam) ? 'array' : typeof filepathParam}, value:`,
        filepathParam
      );

      // Join array segments back into a path
      const rawPath = Array.isArray(filepathParam)
        ? filepathParam.join('/')
        : String(filepathParam || '');
      const filePath = decodeURIComponent(rawPath);

      logger.info(`[AttachmentServer] Request to view: ${filePath}`);

      // Validate path
      const absolutePath = validatePath(filePath);
      if (!absolutePath) {
        res.status(403).send('Forbidden: Invalid file path');
        return;
      }

      // Determine file extension first to decide how to read
      const fileExtension = path.extname(absolutePath).toLowerCase();

      // Render markdown files with mermaid support
      if (fileExtension === '.md' || fileExtension === '.markdown') {
        // Read as UTF-8 for text files
        const fileContent = await readFile(absolutePath, 'utf-8');
        const renderedHtml = renderMarkdown(fileContent);
        const htmlPage = getViewerTemplate({
          title: path.basename(absolutePath),
          content: renderedHtml,
        });

        res.setHeader('Content-Type', 'text/html; charset=utf-8');
        res.send(htmlPage);
      } else {
        // For non-markdown files (images, PDFs, etc.), read as binary
        const fileContent = await readFile(absolutePath);

        // Detect content type and serve
        const contentTypeMap: Record<string, string> = {
          '.png': 'image/png',
          '.jpg': 'image/jpeg',
          '.jpeg': 'image/jpeg',
          '.gif': 'image/gif',
          '.svg': 'image/svg+xml',
          '.pdf': 'application/pdf',
          '.txt': 'text/plain',
        };

        const contentType =
          contentTypeMap[fileExtension] || 'application/octet-stream';
        res.setHeader('Content-Type', contentType);
        res.send(fileContent);
      }
    } catch (error) {
      logger.error(`[AttachmentServer] Error serving file: ${error}`);

      if (
        error &&
        typeof error === 'object' &&
        'code' in error &&
        error.code === 'ENOENT'
      ) {
        res.status(404).send('File not found');
      } else {
        res.status(500).send('Internal server error');
      }
    }
  });

  // Health check endpoint
  app.get('/health', (_req: Request, res: Response) => {
    res.json({ status: 'ok' });
  });

  // Start server
  return new Promise((resolve, reject) => {
    const server = app.listen(port, () => {
      const address = server.address();
      const actualPort =
        typeof address === 'object' && address ? address.port : port;
      logger.info(
        `[AttachmentServer] Server started on http://localhost:${actualPort}`
      );
      resolve(server);
    });

    server.on('error', error => {
      logger.error(`[AttachmentServer] Failed to start server: ${error}`);
      reject(error);
    });
  });
}

/**
 * Stops the attachment server.
 *
 * @param server - HTTP Server instance to stop
 * @returns Promise that resolves when server is closed
 */
export async function stopAttachmentServer(server: Server): Promise<void> {
  return new Promise((resolve, reject) => {
    server.close(error => {
      if (error) {
        logger.error(`[AttachmentServer] Error stopping server: ${error}`);
        reject(error);
      } else {
        logger.info('[AttachmentServer] Server stopped');
        resolve();
      }
    });
  });
}

/**
 * Gets the server's port number.
 *
 * @param server - HTTP Server instance
 * @returns Port number or null if not available
 */
export function getServerPort(server: Server): number | null {
  const address = server.address();
  if (typeof address === 'object' && address) {
    return address.port;
  }
  return null;
}
