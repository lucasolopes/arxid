#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod permute;

#[cfg(feature = "encoding")]
pub mod codec;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "encoding")]
use alloc::string::String;

pub use permute::{deobfuscate, obfuscate, MAX_ID, ROUNDS, WIDTH_BITS};

#[cfg(feature = "encoding")]
pub use codec::{from_base62, to_base62, ALPHABET, CODE_LEN};

/// The specification version this implementation conforms to.
///
/// A change in observable output requires a new spec version and a new set of
/// canonical vectors, never a silent bump. Spec v1 is **withdrawn**: it used 4
/// rounds and an XOR key fold, both of which had measurable defects (see
/// `spec/SPEC.md` section 11). Codes produced under v1 do not decode under v2.
pub const SPEC_VERSION: u32 = 2;

/// A keyed, reversible permutation over the 40-bit id domain.
///
/// Holds the key so callers do not have to thread it through every call. The
/// free functions [`obfuscate`] and [`deobfuscate`] are available when a key
/// is more convenient as an argument.
///
/// ```
/// use arxid::Arxid;
///
/// let a = Arxid::new(0x9E37_79B9_7F4A_7C15);
/// let code = a.obfuscate(1001);
/// assert_ne!(code, 1001);
/// assert_eq!(a.deobfuscate(code), 1001);
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct Arxid {
    key: u64,
}

impl Arxid {
    /// Builds a permutation from a 64-bit key.
    ///
    /// Every bit of the key participates in the key schedule. Use a random key
    /// per deployment, load it from the environment or a secret manager, and
    /// keep it out of source control.
    ///
    /// There is **no tweak**: one key defines one global mapping. If two
    /// resource types share a key, the same id yields the same code in both,
    /// so `/orders/{code}` and `/users/{code}` are linkable. Derive a separate
    /// key per resource type when that matters.
    ///
    /// ```
    /// # use arxid::Arxid;
    /// let a = Arxid::new(0);
    /// assert_eq!(a.deobfuscate(a.obfuscate(0)), 0);
    /// ```
    #[must_use]
    pub const fn new(key: u64) -> Self {
        Self { key }
    }

    /// Builds a permutation from 8 raw key bytes, read **big-endian**.
    ///
    /// The `u64` itself has no endianness, but a key at rest does: the moment
    /// it lives in an environment variable, a config file, or a KMS blob, the
    /// two ends must agree on byte order. This constructor fixes that order so
    /// a key stored by one service is read identically by another, on any
    /// platform.
    ///
    /// ```
    /// # use arxid::Arxid;
    /// let a = Arxid::from_key_bytes([0x9E, 0x37, 0x79, 0xB9, 0x7F, 0x4A, 0x7C, 0x15]);
    /// assert_eq!(a, Arxid::new(0x9E37_79B9_7F4A_7C15));
    /// ```
    #[must_use]
    pub const fn from_key_bytes(bytes: [u8; 8]) -> Self {
        Self {
            key: u64::from_be_bytes(bytes),
        }
    }

    /// Obfuscates an id. Values outside `[0, MAX_ID]` are reduced with
    /// `& MAX_ID`. Never panics.
    ///
    /// ```
    /// # use arxid::{Arxid, MAX_ID};
    /// let a = Arxid::new(42);
    /// assert!(a.obfuscate(7) <= MAX_ID);
    /// ```
    #[must_use]
    pub fn obfuscate(&self, id: u64) -> u64 {
        permute::obfuscate(id, self.key)
    }

    /// Recovers the original id from an obfuscated code. Values outside
    /// `[0, MAX_ID]` are reduced with `& MAX_ID`. Never panics.
    ///
    /// ```
    /// # use arxid::Arxid;
    /// let a = Arxid::new(42);
    /// assert_eq!(a.deobfuscate(a.obfuscate(999)), 999);
    /// ```
    #[must_use]
    pub fn deobfuscate(&self, code: u64) -> u64 {
        permute::deobfuscate(code, self.key)
    }

    /// Obfuscates an id and encodes it as a 7-character base62 string.
    ///
    /// ```
    /// # use arxid::Arxid;
    /// let a = Arxid::new(0x9E37_79B9_7F4A_7C15);
    /// let s = a.obfuscate_str(1001);
    /// assert_eq!(s.len(), 7);
    /// assert_eq!(a.deobfuscate_str(&s), Some(1001));
    /// ```
    #[cfg(feature = "encoding")]
    #[must_use]
    pub fn obfuscate_str(&self, id: u64) -> String {
        codec::to_base62(self.obfuscate(id))
    }

    /// Decodes a 7-character base62 string and recovers the original id.
    ///
    /// Returns `None` when the string is not a valid code (wrong length,
    /// character outside the alphabet, or value above [`MAX_ID`]). Note that
    /// any well-formed code decodes to *some* id: this is a permutation, not a
    /// MAC, and it does not tell you whether the code was produced with this
    /// key.
    ///
    /// ```
    /// # use arxid::Arxid;
    /// let a = Arxid::new(1);
    /// assert_eq!(a.deobfuscate_str("not a code"), None);
    /// ```
    #[cfg(feature = "encoding")]
    #[must_use]
    pub fn deobfuscate_str(&self, s: &str) -> Option<u64> {
        codec::from_base62(s).map(|code| self.deobfuscate(code))
    }
}

/// Redacts the key so it cannot leak through logs or panic messages.
impl core::fmt::Debug for Arxid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Arxid").field("key", &"<redacted>").finish()
    }
}

#[cfg(feature = "zeroize")]
impl Drop for Arxid {
    fn drop(&mut self) {
        use zeroize::Zeroize as _;
        self.key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_and_free_functions_agree() {
        let key = 0x0123_4567_89AB_CDEF;
        let a = Arxid::new(key);
        for id in [0u64, 1, 42, 12345, MAX_ID] {
            assert_eq!(a.obfuscate(id), obfuscate(id, key));
            assert_eq!(a.deobfuscate(id), deobfuscate(id, key));
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn debug_does_not_leak_the_key() {
        let rendered = alloc::format!("{:?}", Arxid::new(0xDEAD_BEEF));
        assert!(!rendered.contains("dead"), "{rendered}");
        assert!(!rendered.contains("3735928559"), "{rendered}");
        assert!(rendered.contains("redacted"), "{rendered}");
    }

    #[cfg(feature = "encoding")]
    #[test]
    fn string_layer_round_trips() {
        let a = Arxid::new(7);
        for id in [0u64, 1, 42, MAX_ID] {
            let s = a.obfuscate_str(id);
            assert_eq!(s.len(), CODE_LEN);
            assert_eq!(a.deobfuscate_str(&s), Some(id));
        }
    }
}
