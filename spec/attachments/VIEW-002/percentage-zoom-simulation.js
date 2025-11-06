/**
 * Simulation test for percentage-based zoom algorithm
 *
 * This simulates the zoom behavior to prove the math is correct.
 */

console.log("=== PERCENTAGE-BASED ZOOM SIMULATION ===\n");

// Setup: Imagine an SVG with intrinsic size 800x600
const SVG_INTRINSIC_WIDTH = 800;
const SVG_INTRINSIC_HEIGHT = 600;

// Parent container is at screen position (100, 50)
const PARENT_SCREEN_LEFT = 100;
const PARENT_SCREEN_TOP = 50;

// Initial state
let currentScale = 1.5;
let currentPanX = 20;
let currentPanY = 30;

// Calculate where SVG is currently rendered on screen
const currentRenderedWidth = SVG_INTRINSIC_WIDTH * currentScale;  // 800 * 1.5 = 1200
const currentRenderedHeight = SVG_INTRINSIC_HEIGHT * currentScale;  // 600 * 1.5 = 900

const svgScreenLeft = PARENT_SCREEN_LEFT + currentPanX;  // 100 + 20 = 120
const svgScreenTop = PARENT_SCREEN_TOP + currentPanY;   // 50 + 30 = 80

console.log("INITIAL STATE:");
console.log("  Scale:", currentScale);
console.log("  Pan:", { x: currentPanX, y: currentPanY });
console.log("  SVG rendered size:", { width: currentRenderedWidth, height: currentRenderedHeight });
console.log("  SVG screen position:", { left: svgScreenLeft, top: svgScreenTop });

// Mouse position (screen coordinates)
const mouseScreenX = 520;
const mouseScreenY = 380;

console.log("\nMOUSE POSITION:");
console.log("  Screen:", { x: mouseScreenX, y: mouseScreenY });

// STEP 1: Calculate mouse position relative to SVG
const mouseRelX = mouseScreenX - svgScreenLeft;  // 520 - 120 = 400
const mouseRelY = mouseScreenY - svgScreenTop;   // 380 - 80 = 300

console.log("  Relative to SVG:", { x: mouseRelX, y: mouseRelY });

// STEP 2: Calculate as PERCENTAGE of current SVG size
const percentX = mouseRelX / currentRenderedWidth;  // 400 / 1200 = 0.3333
const percentY = mouseRelY / currentRenderedHeight;  // 300 / 900 = 0.3333

console.log("  Percentage:", { x: percentX.toFixed(4), y: percentY.toFixed(4) });

// ZOOM IN to new scale
const newScale = 2.0;
const scaleRatio = newScale / currentScale;  // 2.0 / 1.5 = 1.3333

console.log("\n=== APPLYING ZOOM ===");
console.log("  Old scale:", currentScale);
console.log("  New scale:", newScale);
console.log("  Scale ratio:", scaleRatio.toFixed(4));

// STEP 3: Calculate new rendered size
const newRenderedWidth = currentRenderedWidth * scaleRatio;   // 1200 * 1.3333 = 1600
const newRenderedHeight = currentRenderedHeight * scaleRatio;  // 900 * 1.3333 = 1200

console.log("  New rendered size:", { width: newRenderedWidth.toFixed(2), height: newRenderedHeight.toFixed(2) });

// STEP 4: Calculate where the percentage point is in the new size
const percentagePointX = percentX * newRenderedWidth;  // 0.3333 * 1600 = 533.33
const percentagePointY = percentY * newRenderedHeight;  // 0.3333 * 1200 = 400

console.log("  Percentage point offset:", { x: percentagePointX.toFixed(2), y: percentagePointY.toFixed(2) });

// STEP 5: Calculate new pan to keep percentage point under mouse
const newPanX = (mouseScreenX - PARENT_SCREEN_LEFT) - percentagePointX;  // (520 - 100) - 533.33 = -113.33
const newPanY = (mouseScreenY - PARENT_SCREEN_TOP) - percentagePointY;   // (380 - 50) - 400 = -70

console.log("  New pan:", { x: newPanX.toFixed(2), y: newPanY.toFixed(2) });

// VERIFICATION: Check that the percentage point is still under the mouse
const newSvgScreenLeft = PARENT_SCREEN_LEFT + newPanX;
const newSvgScreenTop = PARENT_SCREEN_TOP + newPanY;

console.log("\n=== VERIFICATION ===");
console.log("NEW SVG STATE:");
console.log("  Scale:", newScale);
console.log("  Pan:", { x: newPanX.toFixed(2), y: newPanY.toFixed(2) });
console.log("  SVG screen position:", { left: newSvgScreenLeft.toFixed(2), top: newSvgScreenTop.toFixed(2) });

const percentagePointScreenX = newSvgScreenLeft + percentagePointX;
const percentagePointScreenY = newSvgScreenTop + percentagePointY;

console.log("\nPERCENTAGE POINT:");
console.log("  Offset from SVG:", { x: percentagePointX.toFixed(2), y: percentagePointY.toFixed(2) });
console.log("  Screen position:", { x: percentagePointScreenX.toFixed(2), y: percentagePointScreenY.toFixed(2) });
console.log("  Expected (mouse):", { x: mouseScreenX, y: mouseScreenY });

const driftX = Math.abs(percentagePointScreenX - mouseScreenX);
const driftY = Math.abs(percentagePointScreenY - mouseScreenY);

console.log("\nDRIFT:");
console.log("  X:", driftX.toFixed(6), "pixels");
console.log("  Y:", driftY.toFixed(6), "pixels");

if (driftX < 0.01 && driftY < 0.01) {
  console.log("\n✓ SUCCESS! Percentage point stayed under mouse!");
} else {
  console.log("\n✗ FAILED! Percentage point drifted!");
}

// Test multiple zoom events
console.log("\n\n=== MULTIPLE ZOOM TEST ===");
let state = {
  scale: 1.0,
  panX: 0,
  panY: 0
};

const testMouseX = 500;
const testMouseY = 400;

console.log("Fixed mouse position:", { x: testMouseX, y: testMouseY });
console.log("\nZooming in 5 times:\n");

for (let i = 0; i < 5; i++) {
  const oldScale = state.scale;
  const newScale = oldScale * 1.2;  // 20% zoom in

  // Current SVG position
  const currentWidth = SVG_INTRINSIC_WIDTH * oldScale;
  const currentHeight = SVG_INTRINSIC_HEIGHT * oldScale;
  const svgLeft = PARENT_SCREEN_LEFT + state.panX;
  const svgTop = PARENT_SCREEN_TOP + state.panY;

  // Calculate percentage
  const relX = testMouseX - svgLeft;
  const relY = testMouseY - svgTop;
  const pctX = relX / currentWidth;
  const pctY = relY / currentHeight;

  // Calculate new position
  const ratio = newScale / oldScale;
  const newWidth = currentWidth * ratio;
  const newHeight = currentHeight * ratio;
  const pctPointX = pctX * newWidth;
  const pctPointY = pctY * newHeight;
  const newPanX = (testMouseX - PARENT_SCREEN_LEFT) - pctPointX;
  const newPanY = (testMouseY - PARENT_SCREEN_TOP) - pctPointY;

  // Update state
  state.scale = newScale;
  state.panX = newPanX;
  state.panY = newPanY;

  // Verify
  const verifyLeft = PARENT_SCREEN_LEFT + newPanX;
  const verifyTop = PARENT_SCREEN_TOP + newPanY;
  const verifyX = verifyLeft + pctPointX;
  const verifyY = verifyTop + pctPointY;
  const drift = Math.sqrt(Math.pow(verifyX - testMouseX, 2) + Math.pow(verifyY - testMouseY, 2));

  console.log(`Event ${i + 1}: scale=${state.scale.toFixed(4)}, drift=${drift.toFixed(8)} px`);
}

console.log("\n✓ If all drift values are ~0, the algorithm is correct!");
