# arxid (TypeScript)

`arxid` obfuscates a sequential integer id into a keyed, non-enumerable,
reversible form. It is a balanced Feistel network with an ARX (Add-Rotate-XOR)
round function, format-preserving over 40 bits, plus an optional 7-character
base62 encoding.

This is a **native TypeScript port** of
[`spec/SPEC.md`](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md),
not a WASM wrapper. It is validated against the same canonical vectors as the
Rust reference implementation, so ids obfuscated in either language decode
identically in the other.

ESM, zero runtime dependencies.

## Install

```sh
npm install arxid
```

## Usage

```ts
import { Arxid } from "arxid";

const arxid = new Arxid(0x9e3779b97f4a7c15n); // your own random key

const code = arxid.obfuscate(1001);
arxid.deobfuscate(code); // 1001

const s = arxid.obfuscateStr(1001); // "Cc1W2dQ"
arxid.deobfuscateStr(s);            // 1001
```

Consecutive ids do not produce consecutive codes:

```ts
const arxid = new Arxid(0x0123456789abcdefn);
Math.abs(arxid.obfuscate(100) - arxid.obfuscate(101)); // large, not 1
```

The key can also be passed per call, though `Arxid` is faster for repeated use
because it derives the round subkeys once:

```ts
import { obfuscate, deobfuscate } from "arxid";

deobfuscate(obfuscate(7, 42n), 42n); // 7
```

## Types

**The key is a `bigint`**, because a `u64` does not fit safely in `number`
(2^64 far exceeds 2^53). Values outside `[0, 2^64)` are reduced modulo 2^64.

**Ids and codes are plain `number`s.** The 40-bit domain tops out at
`1099511627775`, well inside the safe integer range, so no big-integer type is
needed for the public values.

```ts
import { MAX_ID, WIDTH_BITS, ROUNDS, CODE_LEN, ALPHABET, SPEC_VERSION } from "arxid";
```

## API

| Export | Signature |
|---|---|
| `Arxid` | `new Arxid(key: bigint)` |
| `Arxid#obfuscate` | `(id: number) => number` |
| `Arxid#deobfuscate` | `(code: number) => number` |
| `Arxid#obfuscateStr` | `(id: number) => string` |
| `Arxid#deobfuscateStr` | `(s: string) => number \| null` |
| `obfuscate` | `(id: number, key: bigint) => number` |
| `deobfuscate` | `(code: number, key: bigint) => number` |
| `toBase62` | `(n: number) => string` |
| `fromBase62` | `(s: string) => number \| null` |

The permutation is **total**: values outside `[0, MAX_ID]` are reduced modulo
2^40 rather than rejected, and nothing throws. Only string decoding can fail,
and it returns `null`:

```ts
fromBase62("abc");      // null - wrong length
fromBase62("!!!!!!!");  // null - invalid character
fromBase62("zzzzzzz");  // null - above MAX_ID
```

## Security

`arxid` is a keyed reversible permutation for id obfuscation. It is **not**
encryption of arbitrary data, it is **not** a MAC (no authentication, no
integrity), and it has **not** undergone independent cryptographic audit.
Non-enumerability is a measured statistical property, not a cryptographic
guarantee. **ID obfuscation is not access control**: never use arxid as the
sole authorization barrier for a resource that must stay secret. Use a random
key per deployment and keep it out of source control.

Note that any well-formed 7-character code decodes to *some* id. A code
decoding successfully does not mean it was issued by you.

## Development

```sh
npm install
npm run typecheck
npm test        # includes the canonical vectors
npm run build
```

## License

MIT. See [`LICENSE`](https://github.com/lucasolopes/arxid/blob/main/LICENSE).
