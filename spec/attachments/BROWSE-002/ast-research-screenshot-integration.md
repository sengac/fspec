# AST Research: Screenshot Integration Points

## Overview
This research analyzes the existing codebase structure to understand where screenshot functionality should be integrated.

## Files Analyzed

### 1. codelet/common/src/web_search.rs
**Purpose**: Defines the WebSearchAction enum shared between facade and tool

**Current Structure**:
- `WebSearchAction` enum with variants: `Search`, `OpenPage`, `FindInPage`, `Other`
- Each variant has optional parameters

**Integration Point**:
- Add `CaptureScreenshot { url, output_path, full_page }` variant

### 2. codelet/tools/src/chrome_browser.rs
**Purpose**: Wrapper around rust-headless-chrome for browser operations

**Existing Functions**:
| Function | Line | Purpose |
|----------|------|---------|
| `new` | 99 | Create ChromeBrowser with config |
| `new_tab` | 143 | Create new browser tab |
| `navigate_and_wait` | 159 | Navigate to URL and wait for load |
| `evaluate_js` | 179 | Execute JavaScript in page |
| `cleanup_tab` | 198 | Close tab after use |

**Integration Point**:
- Add `capture_screenshot` method after `cleanup_tab` (around line 200)
- Use `tab.capture_screenshot()` with `capture_beyond_viewport` parameter
- Return `Vec<u8>` (PNG bytes)

### 3. codelet/tools/src/web_search.rs
**Purpose**: WebSearchTool implementation with action handling

**Existing Functions**:
| Function | Line | Purpose |
|----------|------|---------|
| `normalize_action_type` | 174 | Normalize action type strings |
| `definition` | 254 | Tool JSON schema definition |
| `call` | 323 | Handle action dispatch |
| `perform_web_search` | 393 | Execute search |
| `fetch_page_content` | 436 | Fetch and parse page |
| `find_pattern_in_page` | 489 | Pattern matching in page |

**Integration Points**:
1. `normalize_action_type` (line 174): Add screenshot variations
2. `definition` (line 254): Add capture_screenshot to JSON schema
3. `call` (line 323): Add `CaptureScreenshot` match arm
4. New function: `capture_page_screenshot` (similar to `fetch_page_content`)

## Implementation Order

1. **WebSearchAction enum** (common/src/web_search.rs)
   - Add `CaptureScreenshot` variant

2. **ChromeBrowser** (tools/src/chrome_browser.rs)
   - Add `ScreenshotError` to ChromeError enum
   - Add `capture_screenshot` method using CDP

3. **WebSearchTool** (tools/src/web_search.rs)
   - Update `normalize_action_type` for screenshot variations
   - Update tool `definition` JSON schema
   - Add `CaptureScreenshot` handling in `call`
   - Add `capture_page_screenshot` function

## Notes

- All screenshot operations should go through `with_browser_retry` for connection resilience
- Use existing `navigate_and_wait` before capturing (already handles JS rendering)
- Save to temp directory using `std::env::temp_dir()` with UUID filename
- Return file path in `WebSearchResult.message` field
