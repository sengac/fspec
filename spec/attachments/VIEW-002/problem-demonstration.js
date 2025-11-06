/**
 * Demonstration of the ACTUAL problem from your console logs
 *
 * Your logs show:
 * 1. AFTER_ZOOM: transform applied correctly (expectedPan matches actualTransform)
 * 2. AFTER_HPAN: horizontal pan MODIFIES the pan values
 *
 * This proves the problem is NOT the math, but the MIXING of:
 * - Direct transform manipulation (zoom)
 * - Panzoom library methods (horizontal pan)
 */

// From your first zoom event that had deltaX:
const event1 = {
  mouseScreenX: 878,
  mouseScreenY: 379,
  deltaX: -1,  // ← This triggers horizontal panning
  deltaY: -1,
  containerLeft: 32,
  containerTop: 103,
  currentPanX: -176.27792759017848,
  currentPanY: -77.73482706821522,
  oldScale: 1.2816479241602001,
  newScale: 1.2834258975629114,
};

console.log("=== DEMONSTRATING THE PROBLEM ===\n");

// Step 1: Zoom transform is applied correctly
console.log("Step 1: Apply zoom transform (MATH IS CORRECT)");
const worldX = (event1.mouseScreenX - event1.containerLeft - event1.currentPanX) / event1.oldScale;
const newPanX = (event1.mouseScreenX - event1.containerLeft) - worldX * event1.newScale;
console.log("  Calculated newPanX:", newPanX);
console.log("  Expected from logs: -177.69608848379914");
console.log("  Match:", Math.abs(newPanX - (-177.69608848379914)) < 0.01 ? "✓ YES" : "✗ NO");

// Step 2: Then horizontal panning MODIFIES the values
console.log("\nStep 2: Horizontal panning (THIS IS THE BUG)");
console.log("  After zoom, panX =", newPanX);
console.log("  But logs show AFTER_HPAN: deltaX=-1, pan.x=-176.91584300362544");
console.log("  The horizontal pan CHANGED panX from", newPanX, "to -176.91584300362544");
console.log("  Difference:", (-176.91584300362544 - newPanX).toFixed(6));

// Step 3: This interferes with the next zoom event
console.log("\nStep 3: Next zoom event uses WRONG pan value");
console.log("  Next event expects panX =", newPanX);
console.log("  But actually gets panX = -176.91584300362544 (modified by horizontal pan)");
console.log("  This small error ACCUMULATES over many events");

console.log("\n=== THE ROOT CAUSE ===");
console.log("❌ Problem: Mixing direct transforms (zoom) with panzoom methods (horizontal pan)");
console.log("❌ panzoomInstance.pan() doesn't know about our direct transform changes");
console.log("❌ Each horizontal pan event introduces a small drift");
console.log("❌ Drift accumulates over hundreds of events");

console.log("\n=== THE FIX ===");
console.log("✓ Track our OWN pan/scale values (don't trust panzoom)");
console.log("✓ Apply ALL transforms directly (both zoom AND horizontal pan)");
console.log("✓ Never call panzoomInstance.pan() or panzoomInstance.zoom()");
console.log("✓ Only use element.style.transform for EVERYTHING");
