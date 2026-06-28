//! Wheel-driven zoom/pan + mode-indicator fade for the fullscreen mermaid modal
//! — port of `handleModalWheel` / `showModeIndicator` from
//! `src/server/templates/viewer-scripts.ts` (~336-539).
//!
//! Emitted as a JS fragment inserted inside the modal script's
//! `DOMContentLoaded` closure (see [`super::mermaid_modal::modal_script`]). It
//! relies on closure-scoped state declared in that closure (`panzoomInstance`,
//! `currentModalDiagram`, `ownPanX/ownPanY/ownScale`, `isPanMode`,
//! `modeIndicatorTimeout`, `updateZoomLevel`). The wheel handler implements
//! cursor-centered zoom (zoom point locked per gesture) clamped 0.5x-5x and a
//! horizontal-scroll pan branch; `showModeIndicator` fades the indicator after
//! 2s of inactivity.

/// JS fragment: zoom-session state, `handleModalWheel`, and `showModeIndicator`.
pub const WHEEL_JS: &str = r#"
      let lockedZoomPointX = null;
      let lockedZoomPointY = null;
      let zoomSessionTimeout = null;
      const ZOOM_SESSION_TIMEOUT_MS = 150;

      function handleModalWheel(event) {
        if (!panzoomInstance) return;
        event.preventDefault();
        const deltaX = event.deltaX;
        const deltaY = event.deltaY;
        const deltaMode = event.deltaMode;
        const isPanModifierHeld = isPanMode;
        const currentPan = { x: ownPanX, y: ownPanY };
        const currentScale = ownScale;
        if (isPanModifierHeld) {
          let newX = currentPan.x;
          let newY = currentPan.y;
          if (Math.abs(deltaX) > 0) { newX = currentPan.x - deltaX / currentScale; }
          if (Math.abs(deltaY) > 0) { newY = currentPan.y - deltaY / currentScale; }
          if (Math.abs(deltaX) > 0 || Math.abs(deltaY) > 0) {
            const element = currentModalDiagram;
            if (element) {
              element.style.transform = 'translate(' + newX + 'px, ' + newY + 'px) scale(' + currentScale + ')';
              element.style.transformOrigin = '0 0';
              ownPanX = newX;
              ownPanY = newY;
            }
          }
        } else {
          if (Math.abs(deltaY) > 0) {
            if (lockedZoomPointX === null || lockedZoomPointY === null) {
              lockedZoomPointX = event.clientX;
              lockedZoomPointY = event.clientY;
            }
            const zoomDelta = -deltaY * (deltaMode === 1 ? 0.05 : deltaMode ? 1 : 0.002);
            let newScale = currentScale * Math.pow(2, zoomDelta);
            newScale = Math.max(0.5, Math.min(5, newScale));
            const parentRect = currentModalDiagram.parentElement.getBoundingClientRect();
            const svgRect = currentModalDiagram.getBoundingClientRect();
            const mouseRelativeToSvgX = lockedZoomPointX - svgRect.left;
            const mouseRelativeToSvgY = lockedZoomPointY - svgRect.top;
            const percentageX = mouseRelativeToSvgX / svgRect.width;
            const percentageY = mouseRelativeToSvgY / svgRect.height;
            const scaleRatio = newScale / currentScale;
            const newRenderedWidth = svgRect.width * scaleRatio;
            const newRenderedHeight = svgRect.height * scaleRatio;
            const percentagePointX = percentageX * newRenderedWidth;
            const percentagePointY = percentageY * newRenderedHeight;
            const intrinsicOffsetX = svgRect.left - parentRect.left - currentPan.x;
            const intrinsicOffsetY = svgRect.top - parentRect.top - currentPan.y;
            const newPanX = (lockedZoomPointX - parentRect.left) - percentagePointX - intrinsicOffsetX;
            const newPanY = (lockedZoomPointY - parentRect.top) - percentagePointY - intrinsicOffsetY;
            const element = currentModalDiagram;
            if (element) {
              element.style.transform = 'translate(' + newPanX + 'px, ' + newPanY + 'px) scale(' + newScale + ')';
              element.style.transformOrigin = '0 0';
              ownPanX = newPanX;
              ownPanY = newPanY;
              ownScale = newScale;
            }
            if (zoomSessionTimeout) { clearTimeout(zoomSessionTimeout); }
            zoomSessionTimeout = setTimeout(() => {
              lockedZoomPointX = null;
              lockedZoomPointY = null;
              zoomSessionTimeout = null;
            }, ZOOM_SESSION_TIMEOUT_MS);
          }
          else if (Math.abs(deltaX) > 0) {
            const newPanX = ownPanX - deltaX / ownScale;
            const element = currentModalDiagram;
            if (element) {
              element.style.transform = 'translate(' + newPanX + 'px, ' + ownPanY + 'px) scale(' + ownScale + ')';
              element.style.transformOrigin = '0 0';
              ownPanX = newPanX;
            }
          }
        }
        showModeIndicator();
        updateZoomLevel();
      }

      function showModeIndicator() {
        const indicator = document.querySelector('.mode-indicator');
        if (!indicator) return;
        indicator.style.opacity = '1';
        clearTimeout(modeIndicatorTimeout);
        modeIndicatorTimeout = setTimeout(() => {
          if (!isPanMode) { indicator.style.opacity = '0.5'; }
        }, 2000);
      }
"#;
