# arxid (Rust reference implementation)

`arxid` obfuscates a sequential integer id into a keyed, unordered,
reversible form. It is a balanced Feistel network with an ARX (Add-Rotate-XOR)
round function, format-preserving over 40 bits, plus an optional 7-character
base62 encoding.

This crate is the **reference implementation** of
[`spec/SPEC.md`](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md).
It is a pure library: no server, no HTTP, no network, no I/O.

## Quickstart

```rust
use arxid::Arxid;
# fn key_from_env() -> u64 { 0x0123_4567_89AB_CDEF } // your secret manager

// Load your own random key. Never hardcode one, and never copy a key out of
// documentation - including this page.
let arxid = Arxid::new(key_from_env());

let code = arxid.obfuscate(1001);
assert_eq!(arxid.deobfuscate(code), 1001);

let s = arxid.obfuscate_str(1001); // 7-character base62
assert_eq!(s.len(), 7);
assert_eq!(arxid.deobfuscate_str(&s), Some(1001));
```

Keys stored as raw bytes are read big-endian, so a key written by one service is
read identically by another:

```rust
use arxid::Arxid;

let from_bytes = Arxid::from_key_bytes([0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]);
assert_eq!(from_bytes, Arxid::new(0x0123_4567_89AB_CDEF));
```

Consecutive ids do not produce codes in any recoverable order:

```rust
use arxid::Arxid;

let arxid = Arxid::new(0x0123_4567_89AB_CDEF);
let codes: Vec<u64> = (100..200).map(|id| arxid.obfuscate(id)).collect();
let ascending = codes.windows(2).filter(|w| w[0] < w[1]).count();
assert!((20..80).contains(&ascending)); // no monotone structure
```

Note what is *not* claimed: that neighbouring ids never land on adjacent codes.
That is not a property of a good permutation, and spec v1 was wrong to assert
it. See [SPEC.md §11](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md).

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
arxid = { version = "0.2", default-features = false, features = ["encoding", "zeroize"] }
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

Full threat model:
[SPEC.md §9](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md).

**Recovering an id from a code without the key is not computationally
infeasible.** arxid raises enumeration from "count upward" to "do some work". It
is **not** encryption, **not** a MAC (no authentication, no integrity — every
well-formed in-range code decodes to *some* id), and has had **no independent
cryptographic audit**. **ID obfuscation is not access control.**

Operationally: random key per deployment, loaded from the environment or a
secret manager and never committed; a **separate key per resource type**, since
there is no tweak and one key is one global mapping; and do not echo internal
ids back in responses or errors.

This crate implements **spec v2**. Spec v1 (crate 0.1.x) is withdrawn — it used
4 rounds, which review showed to be separable from a random permutation at ~2^13
chosen queries. Codes issued by 0.1.x do not decode here.

## License

MIT. See [`LICENSE`](https://github.com/lucasolopes/arxid/blob/main/LICENSE).
