//! Optional `wasm-bindgen` surface (feature `wasm`, off by default).
//!
//! This exists so the reference implementation itself can run in a browser. It
//! is NOT how arxid achieves portability: interoperability comes from native
//! ports validated against `/vectors/vectors.json`. Prefer a native port for
//! your language; reach for this only when you specifically want the reference
//! Rust binary.
//!
//! JavaScript numbers cannot hold a `u64` key exactly, so the wasm surface
//! takes the key as a `BigInt` (mapped to `u64`). Ids and codes fit in 40 bits
//! and are exchanged as `f64`-safe integers via `u64` too, which
//! `wasm-bindgen` also maps to `BigInt` for consistency.

use alloc::string::String;

use wasm_bindgen::prelude::wasm_bindgen;

/// A keyed permutation, exposed to JavaScript.
#[wasm_bindgen(js_name = Arxid)]
pub struct WasmArxid {
    inner: crate::Arxid,
}

#[wasm_bindgen(js_class = Arxid)]
impl WasmArxid {
    /// Builds a permutation from a 64-bit key (a JavaScript `BigInt`).
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(key: u64) -> Self {
        Self {
            inner: crate::Arxid::new(key),
        }
    }

    /// Obfuscates an id.
    #[must_use]
    pub fn obfuscate(&self, id: u64) -> u64 {
        self.inner.obfuscate(id)
    }

    /// Recovers the original id from an obfuscated code.
    #[must_use]
    pub fn deobfuscate(&self, code: u64) -> u64 {
        self.inner.deobfuscate(code)
    }

    /// Obfuscates an id and encodes it as a 7-character base62 string.
    #[wasm_bindgen(js_name = obfuscateStr)]
    #[must_use]
    pub fn obfuscate_str(&self, id: u64) -> String {
        self.inner.obfuscate_str(id)
    }

    /// Decodes a 7-character base62 string, returning `undefined` when the
    /// string is not a valid code.
    #[wasm_bindgen(js_name = deobfuscateStr)]
    #[must_use]
    pub fn deobfuscate_str(&self, s: &str) -> Option<u64> {
        self.inner.deobfuscate_str(s)
    }
}
