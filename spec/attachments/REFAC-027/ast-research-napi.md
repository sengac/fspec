# AST research: REFAC-027
```
rpc-types/src/request_context.rs:4:1:#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
rpc-types/src/request_context.rs:13:1:#[cfg_attr(feature = "napi", napi_derive::napi(object, object_from_js = false))]
rpc-types/src/stream_chunk.rs:13:1:#[cfg_attr(
rpc-types/src/session_state.rs:18:1:#[cfg_attr(feature = "napi", napi_derive::napi(object))]
rpc-types/src/session_state.rs:28:1:#[cfg_attr(feature = "napi", napi_derive::napi(object))]
rpc-types/src/session_state.rs:42:1:#[cfg_attr(feature = "napi", napi_derive::napi(object))]
Error: Invalid AST pattern '#[napi] pub fn $F($$$A) $$$B' for rust.

Pattern error: Multiple AST nodes are detected. Please check the pattern source `#[napi] pub fn µF(µµµA) µµµB`.

```
