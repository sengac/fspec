//! Simple NAPI test to prove callback pattern works

use napi::bindgen_prelude::*;

/// Simple test function to verify callback pattern works from TypeScript
#[napi(js_name = "testCallback")]
pub fn test_callback<T: Fn(String) -> Result<String>>(
    input: String,
    callback: T,
) -> Result<String> {
    // Just call the TypeScript callback with the input - simple!
    callback(input)
}