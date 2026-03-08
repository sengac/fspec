/**
 * Feature: spec/features/element-targeted-screenshot.feature
 *
 * This test file validates the acceptance criteria for EXT-015:
 * Element-targeted screenshots via selector or @ref.
 *
 * Tests map directly to Gherkin scenarios in the feature file.
 * All tests should FAIL before implementation (red phase).
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { _resetForTesting } from '../ref-state';
import type { TabScanState } from '../ref-state';

/** Minimal mock tab */
interface MockTab {
  id: number;
  url: string;
  title: string;
  active: boolean;
  windowId: number;
}

const activeTab: MockTab = {
  id: 1,
  url: 'https://example.com',
  title: 'Example',
  active: true,
  windowId: 1,
};

function createMockChromeTabs() {
  return {
    query: vi.fn<(q: Record<string, unknown>) => Promise<MockTab[]>>(),
    update:
      vi.fn<(id: number, p: Record<string, unknown>) => Promise<MockTab>>(),
    remove: vi.fn<(id: number) => Promise<void>>(),
    captureVisibleTab:
      vi.fn<(wid: number, opts: Record<string, unknown>) => Promise<string>>(),
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
    update: vi.fn<(wid: number, p: Record<string, unknown>) => Promise<void>>(),
  };
}

/** 1x1 red pixel PNG as data URL */
function createSmallPngDataUrl(): string {
  return 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==';
}

/** Tracks all OffscreenCanvas instances created during a test */
let canvasInstances: Array<{
  width: number;
  height: number;
  drawImageCalls: Array<unknown[]>;
}> = [];

/**
 * Set up global mocks for Chrome service worker image APIs.
 * These APIs (OffscreenCanvas, createImageBitmap) don't exist in Node.js/Vitest.
 *
 * For element screenshot tests, we need to track drawImage calls to verify
 * cropping coordinates.
 */
function setupImageProcessingMocks(
  imageWidth = 640,
  imageHeight = 480,
  jpegBase64Size = 1000
): void {
  canvasInstances = [];

  const mockBitmap = { width: imageWidth, height: imageHeight, close: vi.fn() };
  (globalThis as Record<string, unknown>).createImageBitmap = vi
    .fn()
    .mockResolvedValue(mockBitmap);

  const fullArea = imageWidth * imageHeight;
  class MockOffscreenCanvas {
    width: number;
    height: number;
    drawImageCalls: Array<unknown[]>;
    constructor(w: number, h: number) {
      this.width = w;
      this.height = h;
      this.drawImageCalls = [];
      canvasInstances.push({
        width: w,
        height: h,
        drawImageCalls: this.drawImageCalls,
      });
    }
    getContext(): { drawImage: ReturnType<typeof vi.fn> } {
      return {
        drawImage: vi.fn((...callArgs: unknown[]) => {
          this.drawImageCalls.push(callArgs);
        }),
      };
    }
    convertToBlob(options?: {
      quality?: number;
    }): Promise<{ arrayBuffer: () => Promise<ArrayBuffer> }> {
      const tileArea = this.width * this.height;
      let scaledSize = Math.max(
        10,
        Math.round(jpegBase64Size * (tileArea / fullArea))
      );
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
  (globalThis as Record<string, unknown>).OffscreenCanvas = MockOffscreenCanvas;
}

function teardownImageProcessingMocks(): void {
  delete (globalThis as Record<string, unknown>).createImageBitmap;
  delete (globalThis as Record<string, unknown>).OffscreenCanvas;
}

describe('Feature: Element-targeted screenshots via selector or @ref', () => {
  describe('Scenario: Full viewport screenshot when selector is omitted (backward compatible)', () => {
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

    it('should return full viewport JPEG when no selector is provided', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab displays a page with viewport 640x480
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot with no selector
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({});

      // @step Then the result contains a JPEG image of the full viewport
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBe(1);
      expect(imageBlocks[0].mimeType).toBe('image/jpeg');

      // @step And the behaviour is identical to the pre-selector implementation
      // No executeScript calls should have been made (no element targeting)
      expect(mockScripting.executeScript).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Element screenshot via @ref scrolls into view and crops', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      _resetForTesting();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      // 640x480 viewport, bitmap produces small JPEG
      setupImageProcessingMocks(640, 480, 1000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
      _resetForTesting();
    });

    it('should scroll element into view and crop to bounding rect via @ref', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const { setTabScanState: setTabScan } = await import('../ref-state');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the agent has run browser_scan_page to populate refs
      const scanState: TabScanState = {
        refs: new Map([
          [
            'e5',
            {
              selector: '#below-fold-element',
              role: 'img',
              name: 'Hero',
              frameId: 0,
            },
          ],
        ]),
        treeText: '',
        timestamp: Date.now(),
      };
      setTabScan(1, scanState);

      // @step And @e5 maps to a CSS selector for an element below the fold
      // (Already set up above — #below-fold-element with frameId 0)

      // Mock executeScript to return bounding rect (element scrolled into view)
      // DPR = 1 for this test
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            x: 50,
            y: 20,
            width: 200,
            height: 100,
            dpr: 1,
          },
        },
      ]);
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot with selector "@e5"
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '@e5' });

      // @step Then the element is scrolled into view
      // executeScript must have been called (for scrollIntoView + getBoundingClientRect)
      expect(mockScripting.executeScript).toHaveBeenCalled();

      // @step And the viewport is captured
      expect(mockTabs.captureVisibleTab).toHaveBeenCalled();

      // @step And the image is cropped to the element bounding rect
      // There should be a crop canvas sized to the element dimensions
      const cropCanvas = canvasInstances.find(
        c => c.width === 200 && c.height === 100
      );
      expect(cropCanvas).toBeDefined();

      // @step And the result contains a JPEG image of the cropped element
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBeGreaterThanOrEqual(1);
      expect(imageBlocks[0].mimeType).toBe('image/jpeg');
    });
  });

  describe('Scenario: Element screenshot via CSS selector', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      _resetForTesting();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      setupImageProcessingMocks(640, 480, 1000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
      _resetForTesting();
    });

    it('should find and crop element by CSS selector', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab has an element matching "#hero-image"
      // Mock executeScript to return bounding rect for #hero-image
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            x: 0,
            y: 0,
            width: 300,
            height: 200,
            dpr: 1,
          },
        },
      ]);
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot with selector "#hero-image"
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '#hero-image' });

      // @step Then the element is scrolled into view
      expect(mockScripting.executeScript).toHaveBeenCalled();

      // @step And the image is cropped to the element bounding rect
      const cropCanvas = canvasInstances.find(
        c => c.width === 300 && c.height === 200
      );
      expect(cropCanvas).toBeDefined();

      // @step And the result contains a JPEG image of the cropped element
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBeGreaterThanOrEqual(1);
      expect(imageBlocks[0].mimeType).toBe('image/jpeg');
    });
  });

  describe('Scenario: Error when @ref is not found', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      _resetForTesting();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      setupImageProcessingMocks(640, 480, 1000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
      _resetForTesting();
    });

    it('should return error when @ref is not found in scan state', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And no scan state exists for the active tab
      // (No setTabScanState called — no refs exist)

      // @step When the agent calls browser_screenshot with selector "@e3"
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '@e3' });

      // @step Then the result is an error with message "Ref @e3 not found. Run browser_scan_page first to scan the page."
      expect(result.isError).toBe(true);
      const textContent = result.content.find(
        (c: Record<string, unknown>) => c.type === 'text'
      );
      expect(textContent).toBeDefined();
      expect((textContent as { text: string }).text).toContain(
        'Ref @e3 not found'
      );
    });

    it('should return descriptive error when ref resolves but element is gone from DOM', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const { setTabScanState: setTabScan } = await import('../ref-state');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the agent has run browser_scan_page to populate refs
      const scanState: TabScanState = {
        refs: new Map([
          [
            'e7',
            {
              selector: '#removed-element',
              role: 'button',
              name: 'Gone',
              frameId: 0,
            },
          ],
        ]),
        treeText: '',
        timestamp: Date.now(),
      };
      setTabScan(1, scanState);

      // @step When querySelector returns null (element removed from DOM since scan)
      mockScripting.executeScript.mockResolvedValue([{ result: null }]);

      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '@e7' });

      // @step Then the error mentions the ref, the resolved selector, and suggests re-scanning
      expect(result.isError).toBe(true);
      const textContent = result.content.find(
        (c: Record<string, unknown>) => c.type === 'text'
      );
      expect(textContent).toBeDefined();
      const text = (textContent as { text: string }).text;
      expect(text).toContain('@e7');
      expect(text).toContain('#removed-element');
      expect(text).toContain('page may have changed');
    });
  });

  describe('Scenario: Error when element has zero visible dimensions', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      _resetForTesting();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      setupImageProcessingMocks(640, 480, 1000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
      _resetForTesting();
    });

    it('should return error when element has zero width/height', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab has an element matching ".hidden-el" with display:none
      // Mock executeScript returning zero-dimension bounding rect
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            dpr: 1,
          },
        },
      ]);

      // @step When the agent calls browser_screenshot with selector ".hidden-el"
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '.hidden-el' });

      // @step Then the result is an error with message "Element has no visible dimensions"
      expect(result.isError).toBe(true);
      const textContent = result.content.find(
        (c: Record<string, unknown>) => c.type === 'text'
      );
      expect(textContent).toBeDefined();
      expect((textContent as { text: string }).text).toContain(
        'Element has no visible dimensions'
      );
    });
  });

  describe('Scenario: Element screenshot in iframe via frame-aware @ref', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      _resetForTesting();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      setupImageProcessingMocks(640, 480, 1000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
      _resetForTesting();
    });

    it('should execute script in correct frameId for iframe elements', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const { setTabScanState: setTabScan } = await import('../ref-state');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the agent has run browser_scan_page which scanned iframes
      // @step And @f2e1 maps to an element inside iframe frame 2
      const scanState: TabScanState = {
        refs: new Map([
          [
            'f2e1',
            {
              selector: '#iframe-button',
              role: 'button',
              name: 'Submit',
              frameId: 2,
            },
          ],
        ]),
        treeText: '',
        timestamp: Date.now(),
      };
      setTabScan(1, scanState);

      // Mock executeScript — the implementation must call with frameIds: [2]
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            x: 10,
            y: 30,
            width: 120,
            height: 40,
            dpr: 1,
          },
        },
      ]);
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot with selector "@f2e1"
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '@f2e1' });

      // @step Then executeScript runs in the correct frameId to get the bounding rect
      expect(mockScripting.executeScript).toHaveBeenCalled();
      const scriptCall = mockScripting.executeScript.mock.calls[0][0] as Record<
        string,
        unknown
      >;
      const target = scriptCall.target as Record<string, unknown>;
      expect(target.frameIds).toEqual([2]);

      // @step And the viewport is captured
      expect(mockTabs.captureVisibleTab).toHaveBeenCalled();

      // @step And the image is cropped to the element position within the viewport
      const cropCanvas = canvasInstances.find(
        c => c.width === 120 && c.height === 40
      );
      expect(cropCanvas).toBeDefined();

      // @step And the result contains a JPEG image of the cropped element
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBeGreaterThanOrEqual(1);
      expect(imageBlocks[0].mimeType).toBe('image/jpeg');
    });
  });

  describe('Scenario: DPR scaling applies device pixel ratio to crop coordinates', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      _resetForTesting();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      // 2x DPR: bitmap is 1280x960 (2x of 640x480 CSS viewport)
      setupImageProcessingMocks(1280, 960, 2000);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
      _resetForTesting();
    });

    it('should multiply CSS rect by DPR for crop coordinates', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the device has a pixel ratio of 2 (Retina display)
      // @step And the active tab has an element with CSS bounding rect (100, 200, 300, 150)
      // executeScript returns CSS pixels + DPR
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            x: 100,
            y: 200,
            width: 300,
            height: 150,
            dpr: 2,
          },
        },
      ]);
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot with selector for that element
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '#test-element' });

      // @step Then the crop uses device pixel coordinates (200, 400, 600, 300) from the captured PNG
      // The crop canvas should be sized to device pixels: 300*2=600, 150*2=300
      const cropCanvas = canvasInstances.find(
        c => c.width === 600 && c.height === 300
      );
      expect(cropCanvas).toBeDefined();

      // Verify the drawImage call uses device pixel source coordinates
      // drawImage(source, sx, sy, sw, sh, dx, dy, dw, dh)
      // sx=200, sy=400, sw=600, sh=300
      expect(cropCanvas!.drawImageCalls.length).toBeGreaterThanOrEqual(1);
      const drawCall = cropCanvas!.drawImageCalls[0];
      // Args: [source, sx=200, sy=400, sw=600, sh=300, dx=0, dy=0, dw=600, dh=300]
      expect(drawCall[1]).toBe(200); // sx = x * dpr
      expect(drawCall[2]).toBe(400); // sy = y * dpr
      expect(drawCall[3]).toBe(600); // sw = width * dpr
      expect(drawCall[4]).toBe(300); // sh = height * dpr

      // @step And the result contains a JPEG image of the correctly scaled crop
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBeGreaterThanOrEqual(1);
      expect(imageBlocks[0].mimeType).toBe('image/jpeg');
    });
  });

  describe('Scenario: Cropped element image passes through resize and tile pipeline', () => {
    let mockTabs: ReturnType<typeof createMockChromeTabs>;
    let mockScripting: ReturnType<typeof createMockChromeScripting>;
    let mockWindows: ReturnType<typeof createMockChromeWindows>;

    beforeEach(() => {
      vi.resetModules();
      _resetForTesting();
      mockTabs = createMockChromeTabs();
      mockScripting = createMockChromeScripting();
      mockWindows = createMockChromeWindows();
      mockTabs.query.mockResolvedValue([activeTab]);
      mockTabs.get.mockResolvedValue(activeTab);
      // Very large element: crop produces 3000x2000 image, large JPEG output
      setupImageProcessingMocks(3000, 2000, 5 * 1024 * 1024);
    });

    afterEach(() => {
      teardownImageProcessingMocks();
      _resetForTesting();
    });

    it('should resize cropped image and tile if needed', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import('../browser-tools');
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab has a very large element producing an image exceeding 1568px on the long edge
      // executeScript returns a large bounding rect
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            x: 0,
            y: 0,
            width: 3000,
            height: 2000,
            dpr: 1,
          },
        },
      ]);
      mockTabs.captureVisibleTab.mockResolvedValue(createSmallPngDataUrl());

      // @step When the agent calls browser_screenshot with selector for that element
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '#large-element' });

      // @step Then the cropped image is resized so the long edge is at most 1568px
      // After crop (3000x2000), resize should scale to fit 1568px long edge
      // 3000 → 1568, 2000 → 2000 * (1568/3000) ≈ 1045
      const resizedCanvas = canvasInstances.find(c => {
        const longEdge = Math.max(c.width, c.height);
        return longEdge <= 1568 && longEdge > 0 && c.width !== 3000;
      });
      expect(resizedCanvas).toBeDefined();
      expect(
        Math.max(resizedCanvas!.width, resizedCanvas!.height)
      ).toBeLessThanOrEqual(1568);

      // @step And the image is encoded as JPEG at 80% quality
      const imageBlocks = result.content.filter(
        (c: Record<string, unknown>) => c.type === 'image'
      );
      expect(imageBlocks.length).toBeGreaterThanOrEqual(1);
      for (const block of imageBlocks) {
        expect(block.mimeType).toBe('image/jpeg');
      }

      // @step And if the result exceeds 800KB it is sliced into vertical tiles
      // With 5MB mock output, tiling should produce multiple tiles
      if (imageBlocks.length > 1) {
        for (const block of imageBlocks) {
          const data = (block as { data: string }).data;
          expect(data.length).toBeLessThan(800 * 1024);
        }
      }
    });
  });
});
