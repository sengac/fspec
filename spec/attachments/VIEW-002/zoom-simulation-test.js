/**
 * Zoom Simulation Test
 *
 * This test simulates the zoom-to-point calculation to prove the math is correct.
 * It uses the actual values from your console logs to verify the formula works.
 */

// Coordinate transformation formulas
function screenToWorld(screenX, containerLeft, panX, scale) {
  return (screenX - containerLeft - panX) / scale;
}

function worldToScreen(worldX, containerLeft, panX, scale) {
  return containerLeft + worldX * scale + panX;
}

function calculateNewPan(mouseScreenX, mouseScreenY, containerLeft, containerTop, currentPanX, currentPanY, oldScale, newScale) {
  // Convert mouse screen position to world coordinates (using OLD scale and pan)
  const worldX = screenToWorld(mouseScreenX, containerLeft, currentPanX, oldScale);
  const worldY = screenToWorld(mouseScreenY, containerTop, currentPanY, oldScale);

  // Calculate new pan to keep the same world point under the cursor
  const newPanX = (mouseScreenX - containerLeft) - worldX * newScale;
  const newPanY = (mouseScreenY - containerTop) - worldY * newScale;

  return { newPanX, newPanY, worldX, worldY };
}

// Test Case 1: Using actual data from your first zoom event
console.log("=== TEST CASE 1: First Zoom Event ===");
const test1 = {
  mouseScreenX: 871,
  mouseScreenY: 358,
  containerLeft: 32,
  containerTop: 103,
  currentPanX: -82.625,
  currentPanY: -77.73482706821522,
  oldScale: 1.1487,
  newScale: 1.1503,
};

const result1 = calculateNewPan(
  test1.mouseScreenX,
  test1.mouseScreenY,
  test1.containerLeft,
  test1.containerTop,
  test1.currentPanX,
  test1.currentPanY,
  test1.oldScale,
  test1.newScale
);

console.log("Input:", test1);
console.log("World point under cursor:", { x: result1.worldX, y: result1.worldY });
console.log("Calculated new pan:", { x: result1.newPanX, y: result1.newPanY });

// Verify: Check that screen position stays the same BEFORE and AFTER zoom
const screenPosBefore = worldToScreen(result1.worldX, test1.containerLeft, test1.currentPanX, test1.oldScale);
const screenPosAfter = worldToScreen(result1.worldX, test1.containerLeft, result1.newPanX, test1.newScale);

console.log("\nVerification:");
console.log("Screen X BEFORE zoom:", screenPosBefore.toFixed(2));
console.log("Screen X AFTER zoom:", screenPosAfter.toFixed(2));
console.log("Difference:", Math.abs(screenPosBefore - screenPosAfter).toFixed(6));
console.log("✓ Point is FIXED:", Math.abs(screenPosBefore - screenPosAfter) < 0.01);

// Test Case 2: Simulate multiple zoom events to check for drift
console.log("\n\n=== TEST CASE 2: Multiple Zoom Events (Drift Detection) ===");

let state = {
  panX: -82.625,
  panY: -77.73482706821522,
  scale: 1.1487,
  mouseX: 871,
  mouseY: 358,
  containerLeft: 32,
  containerTop: 103,
};

console.log("Initial state:", state);
console.log("\nSimulating 10 zoom-in events at the same cursor position:\n");

for (let i = 0; i < 10; i++) {
  const oldScale = state.scale;
  const newScale = oldScale * 1.002; // 0.2% zoom per event

  const result = calculateNewPan(
    state.mouseX,
    state.mouseY,
    state.containerLeft,
    state.containerTop,
    state.panX,
    state.panY,
    oldScale,
    newScale
  );

  // Get world point BEFORE update
  const worldXBefore = screenToWorld(state.mouseX, state.containerLeft, state.panX, state.scale);

  // Update state
  state.panX = result.newPanX;
  state.panY = result.newPanY;
  state.scale = newScale;

  // Get world point AFTER update
  const worldXAfter = screenToWorld(state.mouseX, state.containerLeft, state.panX, state.scale);

  // Verify screen position stayed the same
  const screenBefore = worldToScreen(worldXBefore, state.containerLeft, result.newPanX, newScale);
  const drift = Math.abs(screenBefore - state.mouseX);

  console.log(`Event ${i + 1}: scale=${state.scale.toFixed(4)}, panX=${state.panX.toFixed(2)}, drift=${drift.toFixed(6)}px`);
}

console.log("\n✓ If drift is ~0 for all events, the math is CORRECT");
console.log("✓ If drift accumulates, there's a problem with the formula");

// Test Case 3: Test with your actual logged values
console.log("\n\n=== TEST CASE 3: Using Your Actual Log Values ===");

const actualTest = {
  // From: BEFORE_ZOOM: {"oldScale":1.2816479241602001,"newScale":1.2834258975629114,...}
  mouseScreenX: 878,
  mouseScreenY: 379,
  containerLeft: 32,
  containerTop: 103,
  currentPanX: -176.27792759017848,
  currentPanY: -77.73482706821522,
  oldScale: 1.2816479241602001,
  newScale: 1.2834258975629114,
};

const actualResult = calculateNewPan(
  actualTest.mouseScreenX,
  actualTest.mouseScreenY,
  actualTest.containerLeft,
  actualTest.containerTop,
  actualTest.currentPanX,
  actualTest.currentPanY,
  actualTest.oldScale,
  actualTest.newScale
);

console.log("Expected from logs: newPanX = -177.69608848379914");
console.log("Calculated by formula: newPanX =", actualResult.newPanX);
console.log("Match:", Math.abs(actualResult.newPanX - (-177.69608848379914)) < 0.01 ? "✓ YES" : "✗ NO");

// Verify no drift
const verifyScreenX = worldToScreen(actualResult.worldX, actualTest.containerLeft, actualResult.newPanX, actualTest.newScale);
console.log("\nVerification:");
console.log("Input screen X:", actualTest.mouseScreenX);
console.log("Output screen X:", verifyScreenX.toFixed(2));
console.log("Drift:", Math.abs(verifyScreenX - actualTest.mouseScreenX).toFixed(6), "px");
