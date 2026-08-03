//! Fixed-length base62 string encoding for 40-bit values (SPEC.md section 7).
//!
//! This layer is independent of the permutation: it just serializes a value in
//! `[0, MAX_ID]` as exactly 7 characters and back.

use alloc::string::String;
use alloc::vec::Vec;

use crate::permute::MAX_ID;

/// The base62 alphabet, in normative order: `0-9`, then `A-Z`, then `a-z`.
///
/// This order is part of the contract. It is not the same as some other base62
/// orderings; do not reorder it.
pub const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Length of an encoded code, in characters.
pub const CODE_LEN: usize = 7;

/// Encodes a 40-bit integer as a 7-character base62 string, most significant
/// digit first, left-padded with `'0'`.
///
/// Values above [`MAX_ID`] are silently truncated to 7 digits; callers must
/// reduce to the domain first (everything in this crate's public API already
/// does).
///
/// ```
/// # use arxid::to_base62;
/// assert_eq!(to_base62(0), "0000000");
/// assert_eq!(to_base62(1), "0000001");
/// assert_eq!(to_base62(61), "000000z");
/// assert_eq!(to_base62(62), "0000010");
/// ```
pub fn to_base62(mut n: u64) -> String {
    let mut buf = [b'0'; CODE_LEN];
    let mut i = CODE_LEN;
    while n > 0 && i > 0 {
        i -= 1;
        buf[i] = ALPHABET[(n % 62) as usize];
        n /= 62;
    }
    let owned: Vec<u8> = buf.to_vec();
    // The alphabet is ASCII by construction, so this is always valid UTF-8.
    // `unwrap_or_default` keeps the function total without `unsafe`.
    String::from_utf8(owned).unwrap_or_default()
}

/// Maps one alphabet character to its digit value.
#[inline]
fn val(c: u8) -> Option<u64> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u64),
        b'A'..=b'Z' => Some((c - b'A' + 10) as u64),
        b'a'..=b'z' => Some((c - b'a' + 36) as u64),
        _ => None,
    }
}

/// Decodes a 7-character base62 string back to its integer value.
///
/// Returns `None` (never panics) when the string is not exactly 7 characters,
/// contains a character outside the alphabet, or decodes to a value above
/// [`MAX_ID`].
///
/// ```
/// # use arxid::from_base62;
/// assert_eq!(from_base62("0000000"), Some(0));
/// assert_eq!(from_base62("zzzzzzz"), None); // above MAX_ID
/// assert_eq!(from_base62("abc"), None);     // wrong length
/// assert_eq!(from_base62("!!!!!!!"), None); // invalid character
/// ```
pub fn from_base62(s: &str) -> Option<u64> {
    if s.len() != CODE_LEN {
        return None;
    }
    let mut n: u64 = 0;
    for &c in s.as_bytes() {
        let d = val(c)?;
        n = n.checked_mul(62)?.checked_add(d)?;
    }
    if n <= MAX_ID {
        Some(n)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_base62() {
        for n in [0u64, 1, 61, 62, 3843, 1_000_000, MAX_ID] {
            let s = to_base62(n);
            assert_eq!(s.len(), CODE_LEN);
            assert_eq!(from_base62(&s), Some(n));
        }
    }

    #[test]
    fn alphabet_order_is_normative() {
        assert_eq!(&ALPHABET[..10], b"0123456789");
        assert_eq!(ALPHABET[10], b'A');
        assert_eq!(ALPHABET[35], b'Z');
        assert_eq!(ALPHABET[36], b'a');
        assert_eq!(ALPHABET[61], b'z');
    }

    #[test]
    fn rejects_invalid_char() {
        assert_eq!(from_base62("!!!!!!!"), None);
        assert_eq!(from_base62("00000 0"), None);
    }

    #[test]
    fn rejects_wrong_size() {
        assert_eq!(from_base62(""), None);
        assert_eq!(from_base62("abc"), None);
        assert_eq!(from_base62("aaaaaaaaaaaa"), None);
    }

    #[test]
    fn rejects_out_of_range() {
        assert_eq!(from_base62("zzzzzzz"), None);
        assert_eq!(from_base62(&to_base62(MAX_ID)), Some(MAX_ID));
    }
}
