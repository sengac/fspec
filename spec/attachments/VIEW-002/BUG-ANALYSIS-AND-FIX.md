# VIEW-002 Bug Analysis and Fix

## The Bug

The zoom was drifting to the top-right instead of staying centered on the cursor. Pan values were drifting from `-82` to eventually `-293`.

## Root Cause

The issue was in how we were applying the transform:

```javascript
// BUGGY CODE:
panzoomInstance.zoom(newScale, { animate: false });  // ← This internally adjusts pan!
panzoomInstance.pan(newPanX, newPanY, { animate: false });  // ← Uses stale calculation
```

### What Was Happening:

1. **Step 1**: We read current state
   - `currentPan.x = -82.625`
   - `currentScale = 1.1487`

2. **Step 2**: We calculate new values based on these
   - `newPanX = -83.90` (calculated assuming pan is still -82.625)

3. **Step 3**: We call `zoom(1.1503)`
   - **BUG**: This internally changes pan to ~-83.50 (to keep center fixed)

4. **Step 4**: We call `pan(-83.90)`
   - **BUG**: But -83.90 was calculated for the OLD pan value (-82.625), not -83.50!

5. **Result**: Small drift each frame, accumulating to major displacement

### Visual Proof:

```
Frame 1:  pan.x = -82.62  →  calc newPan = -83.90  →  zoom() sets pan to -83.50  →  pan(-83.90)  →  drift!
Frame 2:  pan.x = -83.03  →  calc newPan = -84.31  →  zoom() sets pan to -83.80  →  pan(-84.31)  →  drift!
...
Frame 200: pan.x = -293   →  MAJOR DRIFT
```

## The Math (Correct)

The coordinate transformation formula is:
```
screenX = containerLeft + worldX * scale + pan.x
```

To keep a world point fixed under the cursor:
```
worldX = (screenX - containerLeft - currentPan.x) / currentScale
newPan.x = (screenX - containerLeft) - worldX * newScale
```

**The math is correct!** The problem was the panzoom library interfering.

## The Fix

Bypass panzoom's `zoom()` method and apply the transform directly:

```javascript
// FIXED CODE:
const element = currentModalDiagram;
element.style.transform = `translate(${newPanX}px, ${newPanY}px) scale(${newScale})`;
element.style.transformOrigin = '0 0';
```

### Why This Works:

1. **No interference**: We don't call `zoom()`, so nothing can modify our calculated pan values
2. **Atomic operation**: Transform is applied in one step
3. **Correct origin**: `transformOrigin = '0 0'` ensures scaling happens from top-left
4. **Direct control**: We apply exactly what our math calculated

## Expected Result

With this fix:
- ✅ The point under the cursor should stay perfectly fixed during zoom
- ✅ No drift - pan values should change predictably based on the formula
- ✅ Works with both mouse wheel and trackpad
- ✅ Horizontal panning still works (separate code path)

## Testing

To verify the fix works:
1. Open a mermaid diagram in fullscreen
2. Position cursor at a specific element
3. Zoom in/out repeatedly
4. The element under the cursor should NOT move
5. Check console logs: `expectedPan` should match `actualTransform`
