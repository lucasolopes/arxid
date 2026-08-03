# arxid

[![CI](https://github.com/lucasolopes/arxid/actions/workflows/ci.yml/badge.svg)](https://github.com/lucasolopes/arxid/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Spec v1](https://img.shields.io/badge/spec-v1%20frozen-informational)](spec/SPEC.md)

**Turn a sequential integer id into a keyed, non-enumerable, reversible code -
identically in any language.**

```
1000  ->  7cAKhgu
1001  ->  Cc1W2dQ
1002  ->  7figrne
```

<sub>(real output, key `0x9E3779B97F4A7C15`)</sub>

arxid is a balanced Feistel network with an ARX (Add-Rotate-XOR) round function,
format-preserving over a 40-bit domain, plus an optional 7-character base62
encoding. It is a pure library: no server, no HTTP, no network, no I/O.

## The problem

Exposing `/orders/1042` tells everyone that orders `1041` and `1043` exist. Your
row count, your growth rate, and your competitors' ability to scrape you all
leak from the URL. Random UUIDs fix the leak but cost you a compact primary key
and a compact URL.

arxid keeps the sequential integer in your database and shows the world a
7-character code that reveals nothing about ordering or volume. It is
reversible with the key, so there is no lookup table and no extra column.

## Quickstart

### Rust

```toml
[dependencies]
arxid = "0.1"
```

```rust
use arxid::Arxid;

let arxid = Arxid::new(0x9E37_79B9_7F4A_7C15); // your own random key

let code = arxid.obfuscate(1001);
assert_eq!(arxid.deobfuscate(code), 1001);

let s = arxid.obfuscate_str(1001);      // 7-character base62
assert_eq!(arxid.deobfuscate_str(&s), Some(1001));
```

### TypeScript

```sh
npm install arxid
```

```ts
import { Arxid } from "arxid";

const arxid = new Arxid(0x9e3779b97f4a7c15n); // bigint: the key is a u64

const code = arxid.obfuscate(1001);
arxid.deobfuscate(code); // 1001

const s = arxid.obfuscateStr(1001);  // 7-character base62
arxid.deobfuscateStr(s);             // 1001
```

## How portability works

The product here is not a library in one language. It is a **frozen
specification plus canonical test vectors**:

- [`spec/SPEC.md`](spec/SPEC.md) - normative, frozen at v1. Every parameter
  (width, rounds, the ARX constants, the golden constant, the key schedule, the
  alphabet and its order, the code length) is part of the contract.
- [`vectors/vectors.json`](vectors/vectors.json) - 61 known-answer tests. Every
  implementation validates against this exact file.

An id obfuscated in Rust deobfuscates identically in TypeScript because both
agree with the vectors. **A port that disagrees with the vectors is a bug in the
port**, not a variation.

Round-trip tests alone are not enough: they are symmetric and hide width and
wrapping bugs. Only the known-answer vectors catch those. See
[`vectors/README.md`](vectors/README.md).

### A note on WASM

The Rust crate has an optional `wasm` feature. It exists so you can run the
reference implementation itself in a browser. **It is not the portability
mechanism.** Interoperability comes from native ports validated against the
vectors, not from shipping one binary everywhere. Prefer the native port for
your language.

## Ports

| Language | Status | Location |
|---|---|---|
| Rust | ✅ reference implementation | [`impl/rust`](impl/rust) |
| TypeScript / JavaScript | ✅ native port | [`impl/ts`](impl/ts) |
| Go | planned | - |
| Python | planned | - |
| C# | planned | - |
| Kotlin / Java | planned | - |
| Ruby | planned | - |
| PHP | planned | - |

Want to add one? See [`CONTRIBUTING.md`](CONTRIBUTING.md). Reimplement the spec
natively, pass the vectors, open a PR. A port that does not pass the vectors is
not merged.

## Parameters

| | |
|---|---|
| Domain | `[0, 2^40 - 1]` = `[0, 1099511627775]` |
| Key | `u64` (effective key space 2^63, see SPEC.md section 2.1) |
| Rounds | 4 (calibrated: smallest count reaching 0.5000 avalanche with full 40/40 coverage) |
| Encoding | base62, fixed 7 characters, alphabet `0-9A-Za-z` in that order |

40 bits holds about 1.1 trillion ids and encodes to exactly 7 characters. The
domain fits inside IEEE-754's safe integer range, so no language needs a
big-integer type for the public values.

## Security

Read this before deploying.

- arxid is a **keyed reversible permutation for id obfuscation** - it defeats
  trivial enumeration of sequential resources. It is **not** encryption of
  arbitrary data.
- It provides **no authentication and no integrity**: it is **not a MAC**. A
  valid-looking output does not prove it was produced by a holder of the key.
  For unforgeability, layer a real MAC on top.
- Non-enumerability is a **measured statistical property** (avalanche/SAC over a
  reduced-round ARX permutation), **not** a cryptographic guarantee. Resistance
  to key recovery depends on the round count and construction. This construction
  has **not** undergone independent cryptographic audit.
- **ID obfuscation is not access control.** Do not use arxid as the sole
  authorization barrier for a resource that must stay secret.
- Each deployment SHOULD use its own random key, kept out of source control.

To report a vulnerability, see [`SECURITY.md`](SECURITY.md).

## Versioning

SemVer, per implementation. The **algorithm** is frozen at spec v1.

Any observable behavior change requires a **new spec version and a new set of
vectors**, never a silent bump. A port producing output different from the
reference is a bug in the port, not an acceptable variation.

## License

MIT. See [`LICENSE`](LICENSE).

The core permutation and base62 encoding were originally written by the same
author for [quark](https://github.com/lucasolopes/quark) and are relicensed
here under MIT by the copyright holder. arxid carries no AGPL-licensed code.
