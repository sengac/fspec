# rust-headless-chrome Full-Page Screenshot Research

## Executive Summary

The `rust-headless-chrome` library (v1.0.20) already has comprehensive screenshot functionality built-in, but **does not expose the Chrome DevTools Protocol's `capture_beyond_viewport` parameter** which is essential for capturing full-page scrollable screenshots.

**Verdict**: Minor enhancement needed to expose existing CDP capability.

---

## Repository Information

- **Repository**: https://github.com/rust-headless-chrome/rust-headless-chrome
- **Crate**: `headless_chrome`
- **Version**: 1.0.20
- **License**: MIT

---

## Current Screenshot Capabilities

### 1. Full Page Screenshots (`Tab::capture_screenshot`)

**Location**: `src/browser/tab/mod.rs:1071-1091`

```rust
pub fn capture_screenshot(
    &self,
    format: Page::CaptureScreenshotFormatOption,  // Jpeg, Png, Webp
    quality: Option<u32>,                          // JPEG quality 0-100
    clip: Option<Page::Viewport>,                  // Optional region to capture
    from_surface: bool,                            // From surface vs view
) -> Result<Vec<u8>>
```

**Supported Formats**:
- `Page::CaptureScreenshotFormatOption::Png`
- `Page::CaptureScreenshotFormatOption::Jpeg`
- `Page::CaptureScreenshotFormatOption::Webp`

### 2. Element-Specific Screenshots (`Element::capture_screenshot`)

**Location**: `src/browser/tab/element/mod.rs:420-431`

```rust
pub fn capture_screenshot(
    &self,
    format: Page::CaptureScreenshotFormatOption,
) -> Result<Vec<u8>>
```

Automatically clips to the element's content-box using `get_box_model()`.

### 3. Background Control

- `set_transparent_background_color()` - For transparent PNGs
- `set_background_color(RGBA)` - Custom background colors

### 4. Screencast (Continuous Capture)

- `start_screencast(format, quality, max_width, max_height, every_nth_frame)`
- `stop_screencast()`
- `ack_screencast(session_id)`

---

## Current Implementation Analysis

### Tab::capture_screenshot Implementation

```rust
// src/browser/tab/mod.rs:1071-1091
pub fn capture_screenshot(
    &self,
    format: Page::CaptureScreenshotFormatOption,
    quality: Option<u32>,
    clip: Option<Page::Viewport>,
    from_surface: bool,
) -> Result<Vec<u8>> {
    let data = self
        .call_method(Page::CaptureScreenshot {
            format: Some(format),
            clip,
            quality,
            from_surface: Some(from_surface),
            capture_beyond_viewport: None,  // ← HARD-CODED TO NONE
            optimize_for_speed: None,       // ← HARD-CODED TO NONE
        })?
        .data;
    base64::prelude::BASE64_STANDARD
        .decode(data)
        .map_err(Into::into)
}
```

**Key Finding**: The `capture_beyond_viewport` parameter is hard-coded to `None`, which means it defaults to `false` and only captures the visible viewport.

---

## Chrome DevTools Protocol Analysis

### CDP `Page.captureScreenshot` Parameters

From `json/browser_protocol.json`:

```json
{
    "name": "captureScreenshot",
    "description": "Capture page screenshot.",
    "parameters": [
        {
            "name": "format",
            "description": "Image compression format (defaults to png).",
            "optional": true,
            "type": "string",
            "enum": ["jpeg", "png", "webp"]
        },
        {
            "name": "quality",
            "description": "Compression quality from range [0..100] (jpeg only).",
            "optional": true,
            "type": "integer"
        },
        {
            "name": "clip",
            "description": "Capture the screenshot of a given region only.",
            "optional": true,
            "$ref": "Viewport"
        },
        {
            "name": "fromSurface",
            "description": "Capture the screenshot from the surface, rather than the view. Defaults to true.",
            "experimental": true,
            "optional": true,
            "type": "boolean"
        },
        {
            "name": "captureBeyondViewport",
            "description": "Capture the screenshot beyond the viewport. Defaults to false.",
            "experimental": true,
            "optional": true,
            "type": "boolean"
        },
        {
            "name": "optimizeForSpeed",
            "description": "Optimize image encoding for speed, not for resulting size (defaults to false)",
            "experimental": true,
            "optional": true,
            "type": "boolean"
        }
    ]
}
```

### Generated Protocol Types

The library auto-generates CDP types from the protocol JSON. The `CaptureScreenshot` struct already includes all parameters:

```rust
// target/debug/build/.../protocol.rs (auto-generated)
pub format: Option<CaptureScreenshotFormatOption>,
pub quality: Option<JsUInt>,
pub clip: Option<Viewport>,
pub from_surface: Option<bool>,
pub capture_beyond_viewport: Option<bool>,  // ← EXISTS but unused
pub optimize_for_speed: Option<bool>,       // ← EXISTS but unused
```

---

## Proposed Solution

### Option 1: Extend Existing Method Signature (Breaking Change)

```rust
pub fn capture_screenshot(
    &self,
    format: Page::CaptureScreenshotFormatOption,
    quality: Option<u32>,
    clip: Option<Page::Viewport>,
    from_surface: bool,
    capture_beyond_viewport: bool,  // NEW PARAMETER
) -> Result<Vec<u8>>
```

**Pros**: Clean API
**Cons**: Breaking change for existing users

### Option 2: Add New Method (Recommended)

```rust
/// Capture a full-page screenshot including content beyond the viewport.
/// 
/// This captures the entire scrollable page, not just the visible viewport.
pub fn capture_full_page_screenshot(
    &self,
    format: Page::CaptureScreenshotFormatOption,
    quality: Option<u32>,
) -> Result<Vec<u8>> {
    let data = self
        .call_method(Page::CaptureScreenshot {
            format: Some(format),
            clip: None,
            quality,
            from_surface: Some(true),
            capture_beyond_viewport: Some(true),  // ← KEY CHANGE
            optimize_for_speed: None,
        })?
        .data;
    base64::prelude::BASE64_STANDARD
        .decode(data)
        .map_err(Into::into)
}
```

**Pros**: Non-breaking, clear intent
**Cons**: API proliferation

### Option 3: Builder Pattern (Most Flexible)

```rust
pub struct ScreenshotOptions {
    pub format: Page::CaptureScreenshotFormatOption,
    pub quality: Option<u32>,
    pub clip: Option<Page::Viewport>,
    pub from_surface: bool,
    pub capture_beyond_viewport: bool,
    pub optimize_for_speed: bool,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            format: Page::CaptureScreenshotFormatOption::Png,
            quality: None,
            clip: None,
            from_surface: true,
            capture_beyond_viewport: false,
            optimize_for_speed: false,
        }
    }
}

impl Tab {
    pub fn capture_screenshot_with_options(&self, options: ScreenshotOptions) -> Result<Vec<u8>> {
        // ... implementation
    }
}
```

**Pros**: Maximum flexibility, forward-compatible
**Cons**: More verbose for simple cases

---

## Full-Page Screenshot Considerations

### Page Size Detection

For full-page screenshots, we may need to:

1. **Query document dimensions**:
```rust
let dimensions = self.evaluate(
    "JSON.stringify({
        width: Math.max(document.body.scrollWidth, document.documentElement.scrollWidth),
        height: Math.max(document.body.scrollHeight, document.documentElement.scrollHeight)
    })",
    false
)?;
```

2. **Optionally set device metrics** to ensure proper rendering:
```rust
self.call_method(Emulation::SetDeviceMetricsOverride {
    width: full_width,
    height: full_height,
    device_scale_factor: 1.0,
    mobile: false,
    // ...
})?;
```

3. **Capture with `capture_beyond_viewport: true`**

4. **Restore original viewport** if needed

### Memory Considerations

Very tall pages could produce large screenshots. Consider:
- Maximum dimension limits
- Memory warnings for large captures
- Optional downscaling parameter

---

## Existing Tests

The library has comprehensive screenshot tests in `tests/simple.rs`:

```rust
#[test]
fn capture_screenshot_png() -> Result<()> { ... }

#[test]
fn capture_screenshot_element() -> Result<()> { ... }

#[test]
fn capture_screenshot_element_box() -> Result<()> { ... }

#[test]
fn capture_screenshot_jpeg() -> Result<()> { ... }

#[test]
fn set_background_color() -> Result<()> { ... }

#[test]
fn set_transparent_background_color() -> Result<()> { ... }
```

These can serve as templates for full-page screenshot tests.

---

## Implementation Effort Estimate

| Task | Effort |
|------|--------|
| Add `capture_full_page_screenshot` method | 1-2 hours |
| Add `ScreenshotOptions` builder (optional) | 2-3 hours |
| Add tests | 1-2 hours |
| Documentation | 1 hour |
| **Total** | **5-8 hours** |

---

## Example Usage (Proposed)

```rust
use headless_chrome::{Browser, LaunchOptions};
use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;

fn main() -> anyhow::Result<()> {
    let browser = Browser::new(LaunchOptions::default_builder().build()?)?;
    let tab = browser.new_tab()?;
    
    tab.navigate_to("https://en.wikipedia.org/wiki/Rust_(programming_language)")?
       .wait_until_navigated()?;
    
    // Viewport-only screenshot (existing behavior)
    let viewport_png = tab.capture_screenshot(
        CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    )?;
    std::fs::write("viewport.png", viewport_png)?;
    
    // Full-page screenshot (new capability)
    let fullpage_png = tab.capture_full_page_screenshot(
        CaptureScreenshotFormatOption::Png,
        None
    )?;
    std::fs::write("fullpage.png", fullpage_png)?;
    
    Ok(())
}
```

---

## References

- [Chrome DevTools Protocol - Page.captureScreenshot](https://chromedevtools.github.io/devtools-protocol/tot/Page/#method-captureScreenshot)
- [rust-headless-chrome Repository](https://github.com/rust-headless-chrome/rust-headless-chrome)
- [Puppeteer fullPage screenshot](https://pptr.dev/api/puppeteer.pagescreenshotoptions#fullpage)
