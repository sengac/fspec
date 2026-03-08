/**
 * Feature: spec/features/screenshot-native-messaging-resilience.feature
 *
 * This test file validates all acceptance criteria for EXT-013:
 * - Native messaging protocol fixes (separate incoming/outgoing limits)
 * - Buffer corruption fix for oversized messages
 * - Screenshot resize, JPEG conversion, and tile slicing
 *
 * Tests map directly to Gherkin scenarios.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Readable } from 'stream';

// ============ Native Messaging Protocol Tests ============

describe('Feature: Screenshot Native Messaging Resilience', () => {
  describe('Scenario: Native message reader accepts messages up to 64 MiB', () => {
    it('should decode and deliver a 2MB message from the extension', async () => {
      // @step Given the native messaging host is running
      const { createNativeMessageReader } = await import(
        /* @vite-ignore */ '../../../host/lib/native-messaging.mjs'
      );

      // @step And the reader is processing messages from the extension
      const inputStream = new Readable({ read() {} });
      const receivedMessages: Record<string, unknown>[] = [];
      createNativeMessageReader(
        inputStream,
        (message: Record<string, unknown>) => {
          receivedMessages.push(message);
        }
      );

      // @step When a 2MB message arrives from the extension via stdin
      const largePayload = 'x'.repeat(2 * 1024 * 1024);
      const message = { type: 'TOOL_RESULT', data: largePayload };
      const jsonBytes = Buffer.from(JSON.stringify(message), 'utf-8');
      const lengthPrefix = Buffer.alloc(4);
      lengthPrefix.writeUInt32LE(jsonBytes.length, 0);
      inputStream.push(Buffer.concat([lengthPrefix, jsonBytes]));

      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the reader decodes and delivers the message successfully
      expect(receivedMessages).toHaveLength(1);
      expect(receivedMessages[0].type).toBe('TOOL_RESULT');

      // @step And subsequent messages are also processed correctly
      const nextMessage = { type: 'NEXT', id: 42 };
      const nextJson = Buffer.from(JSON.stringify(nextMessage), 'utf-8');
      const nextPrefix = Buffer.alloc(4);
      nextPrefix.writeUInt32LE(nextJson.length, 0);
      inputStream.push(Buffer.concat([nextPrefix, nextJson]));

      await new Promise(resolve => setTimeout(resolve, 10));

      expect(receivedMessages).toHaveLength(2);
      expect(receivedMessages[1].type).toBe('NEXT');
    });
  });

  describe('Scenario: Native message reader skips oversized messages without corruption', () => {
    it('should skip a message exceeding 64 MiB and process subsequent messages', async () => {
      // @step Given the native messaging host is running
      const { createNativeMessageReader } = await import(
        /* @vite-ignore */ '../../../host/lib/native-messaging.mjs'
      );

      // @step And the reader is processing messages from the extension
      const inputStream = new Readable({ read() {} });
      const receivedMessages: Record<string, unknown>[] = [];
      createNativeMessageReader(
        inputStream,
        (message: Record<string, unknown>) => {
          receivedMessages.push(message);
        }
      );

      // @step When a message exceeding 64 MiB arrives from the extension
      const oversizedLength = 65 * 1024 * 1024;
      const oversizedPrefix = Buffer.alloc(4);
      oversizedPrefix.writeUInt32LE(oversizedLength, 0);
      const partialBody = Buffer.alloc(100);
      inputStream.push(Buffer.concat([oversizedPrefix, partialBody]));

      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the reader skips the oversized message
      expect(receivedMessages).toHaveLength(0);

      // @step And subsequent messages in the stream are processed correctly
      const validMsg = { type: 'VALID', id: 1 };
      const validJson = Buffer.from(JSON.stringify(validMsg), 'utf-8');
      const validPrefix = Buffer.alloc(4);
      validPrefix.writeUInt32LE(validJson.length, 0);

      const remainingOversized = Buffer.alloc(oversizedLength - 100);
      inputStream.push(
        Buffer.concat([remainingOversized, validPrefix, validJson])
      );

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step And the buffer is not corrupted
      expect(receivedMessages).toHaveLength(1);
      expect(receivedMessages[0].type).toBe('VALID');
    });
  });

  describe('Scenario: Native message encoder preserves 1MB outgoing limit', () => {
    it('should throw when encoding a message larger than 1MB', async () => {
      // @step Given the native messaging host is running
      const { encodeNativeMessage } = await import(
        /* @vite-ignore */ '../../../host/lib/native-messaging.mjs'
      );

      // @step When encoding a message larger than 1MB for the extension
      const largeMessage = { data: 'x'.repeat(1024 * 1024 + 1) };

      // @step Then the encoder throws an error indicating the message exceeds the maximum size
      expect(() => encodeNativeMessage(largeMessage)).toThrow(
        /exceeds max size/i
      );

      // @step And the 1MB limit is enforced for outgoing messages only
      const justUnder = { data: 'x'.repeat(1024 * 1024 - 100) };
      expect(() => encodeNativeMessage(justUnder)).not.toThrow();
    });
  });

  // ============ Browser Screenshot Tests ============
  // These test the browser_screenshot handler which uses OffscreenCanvas
  // and createImageBitmap (Chrome service worker APIs). We mock these since
  // they're not available in Node.js/Vitest.

  /** Minimal mock tab */
  interface MockTab {
    id: number;
    url: string;
    title: string;
    active: boolean;
    windowId: number;
  }

  function createMockChromeTabs() {
    return {
      query: vi.fn<(q: Record<string, unknown>) => Promise<MockTab[]>>(),
      update:
        vi.fn<(id: number, p: Record<string, unknown>) => Promise<MockTab>>(),
      remove: vi.fn<(id: number) => Promise<void>>(),
      captureVisibleTab:
        vi.fn<
          (wid: number, opts: Record<string, unknown>) => Promise<string>
        >(),
      goBack: vi.fn<(id: number) => Promise<void>>(),
      goForward: vi.fn<(id: number) => Promise<void>>(),
      get: vi.fn<(id: number) => Promise<MockTab>>(),
      create: vi.fn(),
      onUpdated: { addListener: vi.fn(), removeListener: vi.fn() },
    };
  }

  function createMockChromeScripting() {
    return {
      executeScript:
        vi.fn<
          (inj: Record<string, unknown>) => Promise<Array<{ result: unknown }>>
        >(),
    };
  }

  function createMockChromeWindows() {
    return {
      update:
        vi.fn<(wid: number, p: Record<string, unknown>) => Promise<void>>(),
    };
  }

  /** 1x1 red pixel PNG as data URL */
  function createSmallPngDataUrl(): string {
    return 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==';
  }

  const activeTab: MockTab = {
    id: 1,
    url: 'https://example.com',
    title: 'Example',
    active: true,
    windowId: 1,
  };

  /** Tracks all OffscreenCanvas instances created during a test */
  let canvasInstances: Array<{ width: number; height: number }> = [];

  /**
   * Set up global mocks for Chrome service worker image APIs.
   * These APIs (OffscreenCanvas, createImageBitmap) don't exist in Node.js/Vitest.
   * The mock scales JPEG output proportionally to canvas area vs full image area,
   * simulating the real behaviour where smaller tiles produce smaller files.
   */
  function setupImageProcessingMocks(
    imageWidth = 640,
    imageHeight = 480,
    jpegBase64Size = 1000
  ): void {
    canvasInstances = [];

    const mockBitmap = {
      width: imageWidth,
      height: imageHeight,
      close: vi.fn(),
    };
    (globalThis as Record<string, unknown>).createImageBitmap = vi
      .fn()
      .mockResolvedValue(mockBitmap);

    const fullArea = imageWidth * imageHeight;
    class MockOffscreenCanvas {
      width: number;
      height: number;
      constructor(w: number, h: number) {
        this.width = w;
        this.height = h;
        canvasInstances.push({ width: w, height: h });
      }
      getContext(): { drawImage: ReturnType<typeof vi.fn> } {
        return { drawImage: vi.fn() };
      }
      convertToBlob(options?: {
        quality?: number;
      }): Promise<{ arrayBuffer: () => Promise<ArrayBuffer> }> {
        const tileArea = this.width * this.height;
        let scaledSize = Math.max(
          10,
          Math.round(jpegBase64Size * (tileArea / fullArea))
        );
        // Simulate quality reduction producing smaller output
        if (options?.quality !== undefined && options.quality < 0.8) {
          scaledSize = Math.round(scaledSize * (options.quality / 0.8));
        }
        const data = new Uint8Array(scaledSize);
        for (let i = 0; i < scaledSize; i++) {
          data[i] = 65 + (i % 26);
        }
        return Promise.resolve({
          arrayBuffer: () => Promise.resolve(data.buffer),
        });
      }
    }
    (globalThis as Record<string, unknown>).OffscreenCanvas =
      MockOffscreenCanvas;
  }

  function teardownImageProcessingMocks(): void {
    delete (globalThis as Record<string, unknown>).createImageBitmap;
    delete (globalThis as Record<string, unknown>).OffscreenCanvas;
  }

  describe('Scenario: Small screenshot returns a single JPEG image', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      setupImageProcessingMocks(640, 480, 1000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
    });

    it('should return a single JPEG content block for a small screenshot', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab displays a simple page with viewport 640x480
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({});

      // @step Then the result contains exactly 1 image content block
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBe(1);

      // @step And the image mimeType is "image/jpeg"
      expect(imageBlocks[0].mimeType).toBe('image/jpeg');

      // @step And the image base64 data is under 800KB
      const base64Data = (imageBlocks[0] as { data: string }).data;
      expect(base64Data.length).toBeLessThan(800 * 1024);
    });
  });

  describe('Scenario: Large screenshot is resized to fit within 1568px on long edge', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      setupImageProcessingMocks(1728, 992, 5000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
    });

    it('should resize a large screenshot and return JPEG content', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab displays a complex page with viewport 1728x992
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({});

      // @step Then the captured image is resized so the long edge is at most 1568px
      const fullCanvas = canvasInstances[0];
      expect(fullCanvas).toBeDefined();
      const resizedLongEdge = Math.max(fullCanvas.width, fullCanvas.height);
      expect(resizedLongEdge).toBeLessThanOrEqual(1568);
      // 1728x992 → scale = 1568/1728 ≈ 0.9074 → 1568x900
      expect(fullCanvas.width).toBe(1568);
      expect(fullCanvas.height).toBe(Math.round(992 * (1568 / 1728)));

      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBeGreaterThanOrEqual(1);

      // @step And the aspect ratio is preserved
      const expectedRatio = 1728 / 992;
      const actualRatio = fullCanvas.width / fullCanvas.height;
      expect(Math.abs(expectedRatio - actualRatio)).toBeLessThan(0.02);

      // @step And the result contains image content blocks with mimeType "image/jpeg"
      for (const block of imageBlocks) {
        expect(block.mimeType).toBe('image/jpeg');
      }
    });
  });

  describe('Scenario: Very tall screenshot is sliced into multiple tiles', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      // Very tall image: 1568x4000, mock produces 5MB at full size.
      // Tiles are proportionally smaller; quality fallback further shrinks them.
      setupImageProcessingMocks(1568, 4000, 5 * 1024 * 1024);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
    });

    it('should return JPEG tiles each under 800KB', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And a full-page capture produces a very tall image before resize
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({});

      // @step Then the result contains multiple image content blocks
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBeGreaterThanOrEqual(2);

      // @step And each image content block has mimeType "image/jpeg"
      for (const block of imageBlocks) {
        expect(block.mimeType).toBe('image/jpeg');
      }

      // @step And each image base64 data is under 800KB
      for (const block of imageBlocks) {
        const data = (block as { data: string }).data;
        expect(data.length).toBeLessThan(800 * 1024);
      }
    });
  });
});
