# arxid (Rust reference implementation)

`arxid` obfuscates a sequential integer id into a keyed, non-enumerable,
reversible form. It is a balanced Feistel network with an ARX (Add-Rotate-XOR)
round function, format-preserving over 40 bits, plus an optional 7-character
base62 encoding.

This crate is the **reference implementation** of
[`spec/SPEC.md`](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md).
It is a pure library: no server, no HTTP, no network, no I/O.

## Quickstart

```rust
use arxid::Arxid;

let arxid = Arxid::new(0x9E37_79B9_7F4A_7C15); // your own random key

let code = arxid.obfuscate(1001);
assert_eq!(arxid.deobfuscate(code), 1001);

let s = arxid.obfuscate_str(1001); // 7-character base62
assert_eq!(s.len(), 7);
assert_eq!(arxid.deobfuscate_str(&s), Some(1001));
```

Consecutive ids do not produce consecutive codes:

```rust
use arxid::Arxid;

let arxid = Arxid::new(0x0123_4567_89AB_CDEF);
let a = arxid.obfuscate(100);
let b = arxid.obfuscate(101);
assert!(a.abs_diff(b) > 1);
```

The key can also be passed per call:

```rust
use arxid::{obfuscate, deobfuscate};

let key = 42;
assert_eq!(deobfuscate(obfuscate(7, key), key), 7);
```

## Domain

The permutation is defined over `[0, MAX_ID]` where `MAX_ID = 2^40 - 1`.
Anything outside is reduced with `& MAX_ID`, so the functions are total and
never panic:

```rust
use arxid::{obfuscate, MAX_ID};

assert_eq!(obfuscate(u64::MAX, 1), obfuscate(MAX_ID, 1));
```

String decoding is stricter and returns `None` instead of panicking:

```rust
use arxid::from_base62;

assert_eq!(from_base62("abc"), None);      // wrong length
assert_eq!(from_base62("!!!!!!!"), None);  // invalid character
assert_eq!(from_base62("zzzzzzz"), None);  // above MAX_ID
```

## Features

| Feature | Default | What it does |
|---|---|---|
| `std` | on | Links `std`. Turn off for `no_std` targets. |
| `alloc` | on (via `std`) | Enables allocation. Required by `encoding`. |
| `encoding` | on | The base62 string layer (`obfuscate_str`, `to_base62`, ...). |
| `zeroize` | on | Wipes the key from memory on drop. |
| `wasm` | off | Exposes the core through `wasm-bindgen`. |

`no_std` is supported: the core permutation needs only `core`, and the base62
layer needs only `alloc`.

```toml
[dependencies]
arxid = { version = "0.1", default-features = false, features = ["encoding", "zeroize"] }
```

`wasm` is an optional **build target**, not the portability mechanism.
Interoperability across languages comes from native ports validated against
`/vectors/vectors.json`.

## Interoperability

The canonical test vectors in `/vectors/vectors.json` are the interop contract.
`tests/vectors.rs` checks every one of them, in both directions. A port that
disagrees with those vectors is a bug in the port.

Regenerate the vectors from this crate with:

```text
cargo run --example gen_vectors > ../../vectors/vectors.json
```

## Security

`arxid` is a keyed reversible permutation for id obfuscation. It is **not**
encryption of arbitrary data, it is **not** a MAC (no authentication, no
integrity), and it has **not** undergone independent cryptographic audit.
Non-enumerability is a measured statistical property, not a cryptographic
guarantee. **ID obfuscation is not access control**: never use arxid as the
sole authorization barrier for a resource that must stay secret. Use a random
key per deployment and keep it out of source control.

## License

MIT. See [`LICENSE`](https://github.com/lucasolopes/arxid/blob/main/LICENSE).
