/**
 * fspec Browser Agent - Iframe Scanner
 *
 * Multi-frame scanning logic for iframe-aware DOM scanning.
 * Discovers frames via chrome.webNavigation.getAllFrames,
 * performs two-pass injection for frame-to-DOM correlation,
 * scans each frame, and merges results into a unified tree.
 *
 * Implemented by: LOCATE-009
 */

import type { RawElement } from './dom-scanner';
import type { RefEntry } from './ref-state';
import type { FrameInfo, ChromeScriptingForTools } from './browser-tools-types';

/** Default limit on how many iframes to scan */
export const DEFAULT_MAX_FRAMES = 10;

/** Scan result from a single frame's scanPageDOM call */
export interface FrameScanResult {
  elements: RawElement[];
  metadata: {
    url: string;
    title: string;
    viewportWidth: number;
    viewportHeight: number;
    totalElements: number;
  };
}

/** Check if a frame URL is scannable */
export function isScannableFrame(url: string): boolean {
  if (url.startsWith('chrome-extension://') || url.startsWith('chrome://')) {
    return false;
  }
  if (
    url.startsWith('http://') ||
    url.startsWith('https://') ||
    url === 'about:blank' ||
    url === 'about:srcdoc'
  ) {
    return true;
  }
  return false;
}

/**
 * Prioritize frames for scanning when count exceeds maxFrames.
 * Same-origin frames first, then by URL length (proxy for larger/more important).
 */
export function prioritizeFrames(
  subframes: FrameInfo[],
  mainUrl: string,
  maxFrames: number
): { scanned: FrameInfo[]; skipped: FrameInfo[] } {
  if (subframes.length <= maxFrames) {
    return { scanned: subframes, skipped: [] };
  }
  const mainOrigin = new URL(mainUrl).origin;
  const sorted = [...subframes].sort((a, b) => {
    const aOrigin = a.url.startsWith('http') ? new URL(a.url).origin : '';
    const bOrigin = b.url.startsWith('http') ? new URL(b.url).origin : '';
    const aSame = aOrigin === mainOrigin ? 0 : 1;
    const bSame = bOrigin === mainOrigin ? 0 : 1;
    if (aSame !== bSame) {
      return aSame - bSame;
    }
    // Longer URL suggests more content (heuristic)
    return b.url.length - a.url.length;
  });
  return {
    scanned: sorted.slice(0, maxFrames),
    skipped: sorted.slice(maxFrames),
  };
}

/**
 * Inject frameId markers into each subframe (first pass of two-pass injection).
 */
export async function injectFrameMarkers(
  scripting: ChromeScriptingForTools,
  tabId: number,
  frames: FrameInfo[]
): Promise<void> {
  for (const frame of frames) {
    try {
      await scripting.executeScript({
        target: {
          tabId,
          frameIds: [frame.frameId],
        } as chrome.scripting.InjectionTarget,
        args: [frame.frameId],
        func: (fid: number) => {
          (globalThis as Record<string, unknown>).__fspec_frameId = fid;
        },
      } as chrome.scripting.ScriptInjection<[number], void>);
    } catch {
      // Frame may have been destroyed or is non-injectable
    }
  }
}

/**
 * Scan multiple frames and return per-frame scan results.
 */
export async function scanFrames(
  scripting: ChromeScriptingForTools,
  tabId: number,
  frames: FrameInfo[],
  scanFunc: (interactiveMode: boolean, scope?: string) => FrameScanResult,
  interactive: boolean
): Promise<Map<number, FrameScanResult>> {
  const results = new Map<number, FrameScanResult>();
  await Promise.all(
    frames.map(async frame => {
      try {
        const frameResults = await scripting.executeScript({
          target: {
            tabId,
            frameIds: [frame.frameId],
          } as chrome.scripting.InjectionTarget,
          args: [interactive, null] as [boolean, string | null],
          func: scanFunc,
        });
        const scanResult = frameResults[0]?.result as FrameScanResult | null;
        if (scanResult) {
          results.set(frame.frameId, scanResult);
        }
      } catch {
        // Frame injection failed — skip gracefully
      }
    })
  );
  return results;
}

/** Build a lookup map from frameId → FrameInfo. */
function buildFrameMap(frames: FrameInfo[]): Map<number, FrameInfo> {
  const map = new Map<number, FrameInfo>();
  for (const f of frames) {
    map.set(f.frameId, f);
  }
  return map;
}

/**
 * Compute the nesting depth of a frame from its parentFrameId chain.
 * Main frame (parentFrameId -1) → depth 0.
 * Direct child of main → depth 0 (iframe container sits at tree root level).
 * Child of a child → depth 2 (nested inside the parent iframe's content at depth 1).
 *
 * Accepts a pre-built frameMap to avoid rebuilding on every call.
 */
function computeFrameDepth(
  frameId: number,
  frameMap: Map<number, FrameInfo>
): number {
  let depth = 0;
  let current = frameMap.get(frameId);
  while (current && current.parentFrameId >= 0) {
    const parent = frameMap.get(current.parentFrameId);
    if (
      !parent ||
      parent.frameId === 0 ||
      parent.frameType === 'outermost_frame'
    ) {
      break;
    }
    // Each nesting level adds 2: 1 for the parent iframe container + 1 for its content
    depth += 2;
    current = parent;
  }
  return depth;
}

/**
 * Merge main frame elements with iframe scan results into a unified tree.
 * Returns merged elements and ref map.
 *
 * Iframe containers are placed after main frame content. Nested iframes
 * use depth computed from the parentFrameId chain so the tree shows
 * correct indentation for iframe-inside-iframe structures.
 */
export function mergeFrameResults(
  mainElements: RawElement[],
  framesToScan: FrameInfo[],
  frameScanResults: Map<number, FrameScanResult>,
  skippedFrames: FrameInfo[],
  nonScannableFrames: FrameInfo[],
  allFrames?: FrameInfo[]
): { mergedElements: RawElement[]; refs: Map<string, RefEntry> } {
  let refCounter = 1;
  const refs = new Map<string, RefEntry>();
  const mergedElements: RawElement[] = [];
  const frameDepthSource = allFrames ?? [
    ...framesToScan,
    ...skippedFrames,
    ...nonScannableFrames,
  ];
  const frameMap = buildFrameMap(frameDepthSource);

  // Build a URL→FrameInfo lookup for matching main-frame IFRAME elements to frames
  const urlToFrame = new Map<string, FrameInfo>();
  for (const f of [...framesToScan, ...skippedFrames, ...nonScannableFrames]) {
    // First frame with this URL wins (document-order tiebreaker for duplicates)
    if (!urlToFrame.has(f.url)) {
      urlToFrame.set(f.url, f);
    }
  }

  // Track which frames were already spliced into the tree via main-frame IFRAME elements
  const splicedFrameIds = new Set<number>();

  // Main frame elements — when we encounter an IFRAME element, splice its content inline
  for (const element of mainElements) {
    if (element.interactive) {
      const refKey = `e${refCounter++}`;
      refs.set(refKey, {
        selector: element.selector,
        role: element.role,
        name: element.name,
        frameId: 0,
      });
      element.ref = refKey;
    }
    mergedElements.push(element);

    // If this is an IFRAME element from the main frame scan, splice frame content here
    if (element.tagName === 'IFRAME') {
      const src = element.attributes.src ?? element.name ?? '';
      const matchedFrame = urlToFrame.get(src);
      if (matchedFrame && !splicedFrameIds.has(matchedFrame.frameId)) {
        splicedFrameIds.add(matchedFrame.frameId);
        const frameResult = frameScanResults.get(matchedFrame.frameId);
        if (frameResult) {
          const baseDepth = element.depth;
          let frameRefCounter = 1;
          for (const child of frameResult.elements) {
            if (child.interactive) {
              const refKey = `f${matchedFrame.frameId}e${frameRefCounter++}`;
              refs.set(refKey, {
                selector: child.selector,
                role: child.role,
                name: child.name,
                frameId: matchedFrame.frameId,
              });
              child.ref = refKey;
            }
            mergedElements.push({
              ...child,
              depth: child.depth + baseDepth + 1,
            });
          }
        }
      }
    }
  }

  // Sort remaining (unspliced) frames by depth so parents appear before children
  const sortedFramesToScan = [...framesToScan]
    .filter(f => !splicedFrameIds.has(f.frameId))
    .sort((a, b) => {
      const aDepth = computeFrameDepth(a.frameId, frameMap);
      const bDepth = computeFrameDepth(b.frameId, frameMap);
      return aDepth - bDepth;
    });

  // Append unspliced scanned iframes with their children
  for (const frame of sortedFramesToScan) {
    const frameResult = frameScanResults.get(frame.frameId);
    const baseDepth = computeFrameDepth(frame.frameId, frameMap);
    mergedElements.push({
      tagName: 'IFRAME',
      role: 'iframe',
      name: frame.url,
      selector: '',
      interactive: false,
      depth: baseDepth,
      attributes: { src: frame.url },
    });

    if (frameResult) {
      let frameRefCounter = 1;
      for (const element of frameResult.elements) {
        if (element.interactive) {
          const refKey = `f${frame.frameId}e${frameRefCounter++}`;
          refs.set(refKey, {
            selector: element.selector,
            role: element.role,
            name: element.name,
            frameId: frame.frameId,
          });
          element.ref = refKey;
        }
        mergedElements.push({
          ...element,
          depth: element.depth + baseDepth + 1,
        });
      }
    }
  }

  // Skipped iframes (only those not already in tree from main frame scan)
  for (const frame of skippedFrames) {
    if (splicedFrameIds.has(frame.frameId)) {
      continue;
    }
    const baseDepth = computeFrameDepth(frame.frameId, frameMap);
    mergedElements.push({
      tagName: 'IFRAME',
      role: 'iframe',
      name: `${frame.url} [skipped]`,
      selector: '',
      interactive: false,
      depth: baseDepth,
      attributes: { src: frame.url },
    });
  }

  // Non-scannable iframes (only those not already in tree from main frame scan)
  for (const frame of nonScannableFrames) {
    if (splicedFrameIds.has(frame.frameId)) {
      continue;
    }
    const baseDepth = computeFrameDepth(frame.frameId, frameMap);
    mergedElements.push({
      tagName: 'IFRAME',
      role: 'iframe',
      name: frame.url,
      selector: '',
      interactive: false,
      depth: baseDepth,
      attributes: { src: frame.url },
    });
  }

  return { mergedElements, refs };
}
