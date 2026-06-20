//! WASM boundary. The browser workbench only ever sees this typed surface;
//! everything below it (crypto, compression, parsing) is hidden behind the
//! crate API. This module compiles only for `wasm32` targets.

use wasm_bindgen::prelude::*;

/// Returns the codec version. Minimal entry point that proves the
/// `wasm-bindgen` boundary compiles; richer `openBundle` / `packSubmission`
/// bindings are layered on as the codec lands.
#[wasm_bindgen]
pub fn fcb_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
