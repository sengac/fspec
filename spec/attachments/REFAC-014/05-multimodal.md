# Module 5: `multimodal.rs`

**Path**: `codelet/cli/src/interactive/multimodal.rs`  
**Estimated lines**: ~80  
**Responsibility**: BRIDGE-007 / EXT-016 — BridgeImage struct and user content building with pixel dimension validation.

---

## Items to Extract

### Types:

| Type | Current Lines | Description |
|------|--------------|-------------|
| `BridgeImage` | 541–546 | Struct: `{ data: String, media_type: String }` |

### Functions:

| Function | Current Lines | Signature |
|----------|--------------|-----------|
| `build_user_content_with_images` | 554–601 | `pub fn build_user_content_with_images(prompt: &str, images: Option<Vec<BridgeImage>>) -> OneOrMany<UserContent>` |

---

## Dependencies

```rust
use rig::message::{UserContent, ImageMediaType};
use rig::one_or_many::OneOrMany;
use codelet_tools::image_dimensions;  // extract_dimensions_from_base64, exceeds_pixel_limit, format_dimension_error
use tracing::warn;
```

---

## Notes

`build_user_content_with_images` handles:
1. Text-only prompts (no images) → simple `OneOrMany::one(UserContent::text(prompt))`
2. Mixed text + images → validates each image's pixel dimensions via `codelet_tools::image_dimensions`
3. Oversized images → replaced with text error message (Layer 3 defense-in-depth)
4. Media type mapping: jpeg/png/gif/webp → `ImageMediaType` enum

The `BridgeImage` struct is used by `run_agent_stream_with_images` and the NAPI agent loop.

---

## Re-exports in `mod.rs`

```rust
pub use multimodal::{BridgeImage, build_user_content_with_images};
```

---

## Test Coverage

- `image_content_recovery_test.rs` — imports `build_user_content_with_images`
- `stream_loop_pause_test.rs` — may reference `BridgeImage`
