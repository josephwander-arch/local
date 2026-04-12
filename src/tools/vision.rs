//! Vision tools - screenshots, OCR, image analysis
//! Delegates to vision-core library. Uses block_on since mcp-windows is sync.
// NAV: TOC at line 18 | 2 fn | 0 struct | 2026-02-06

use serde_json::Value;

pub fn get_definitions() -> Vec<Value> {
    vision_core::get_local_definitions()
}

pub fn execute(name: &str, args: &Value) -> Value {
    // vision-core is async (OCR uses Windows async APIs)
    // mcp-windows main loop is sync, so use block_on
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(vision_core::execute(name, args))
}

// === FILE NAVIGATION ===
// Generated: 2026-02-06T18:08:42
// Total: 15 lines | 2 functions | 0 structs | 0 constants
//
// IMPORTS: serde_json
//
// FUNCTIONS:
//   pub +get_definitions: 6-8
//   pub +execute: 10-15
//
// === END FILE NAVIGATION ===