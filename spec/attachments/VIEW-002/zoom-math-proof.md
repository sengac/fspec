# Zoom-to-Point Mathematical Proof

## Coordinate System

For a panzoom transform, the relationship between world coordinates (wx, wy) and screen coordinates (sx, sy) is:

```
sx = containerLeft + wx * scale + pan.x
sy = containerTop + wy * scale + pan.y
```

Solving for world coordinates:
```
wx = (sx - containerLeft - pan.x) / scale
wy = (sy - containerTop - pan.y) / scale
```

## Goal

Keep the same world point (wx, wy) under the cursor at screen position (sx, sy) before and after zoom.

## Test Case - Manual Calculation

### Initial State (Event 1)
- Mouse screen position: (871, 358)
- Container left: 32
- Current pan.x: -82.625
- Current scale: 1.1487
- New scale: 1.1503

### Step 1: Calculate world coordinate under cursor
```
wx = (871 - 32 - (-82.625)) / 1.1487
   = (871 - 32 + 82.625) / 1.1487
   = 921.625 / 1.1487
   = 802.32
```

### Step 2: Calculate new pan to keep wx under cursor
```
871 = 32 + 802.32 * 1.1503 + newPan.x
newPan.x = 871 - 32 - (802.32 * 1.1503)
newPan.x = 839 - 922.90
newPan.x = -83.90
```

### Verification: Does the point stay fixed?

**Before zoom:**
```
sx = 32 + 802.32 * 1.1487 + (-82.625)
   = 32 + 921.625 - 82.625
   = 871 ✓
```

**After zoom:**
```
sx = 32 + 802.32 * 1.1503 + (-83.90)
   = 32 + 922.90 - 83.90
   = 871 ✓
```

## The Math Works!

The formula is mathematically correct. The world point SHOULD stay fixed at the cursor position.

## So Why Is It Drifting?

Looking at the actual logs:
- Event 1: pan.x goes from -82.62 to -83.90 (drift: -1.28)
- Event 2: pan.x goes from -83.03 to -84.31 (drift: -1.28)
- ...continues drifting...

Each zoom event adds a small drift. By event 200+, pan.x has drifted to -293!

## Hypothesis: Transform Origin Issue

Panzoom might be applying transforms with a transform-origin that's not at (0, 0). Or there's interference between the zoom() and pan() calls.

## Alternative Hypothesis: State Sync Issue

When we call `panzoomInstance.getPan()` and `panzoomInstance.getScale()`, we might be getting stale values that don't reflect the actual DOM state.

## Solution Needed

We need to either:
1. Use panzoom's built-in transform matrix directly
2. Calculate the transform in a single operation instead of separate zoom() + pan()
3. Use a different approach that accounts for panzoom's internal state management
