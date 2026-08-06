# arxid (TypeScript)

`arxid` obfuscates a sequential integer id into a keyed, unordered,
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

// Load your own random key. Never hardcode one, and never copy a key out of
// documentation - including this page.
const arxid = new Arxid(BigInt(process.env.ARXID_KEY!));

const code = arxid.obfuscate(1001);
arxid.deobfuscate(code); // 1001

const s = arxid.obfuscateStr(1001); // 7-character base62
arxid.deobfuscateStr(s);            // 1001
```

Keys stored as raw bytes are read big-endian, so a key written by a Rust service
is read identically here:

```ts
Arxid.fromKeyBytes(new Uint8Array([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));
```

Consecutive ids do not produce codes in any recoverable order:

```ts
const arxid = new Arxid(0x0123456789abcdefn);
const codes = Array.from({ length: 100 }, (_, i) => arxid.obfuscate(100 + i));
// no monotone structure: roughly half the steps ascend
```

Note what is *not* claimed: that neighbouring ids never land on adjacent codes.
That is not a property of a good permutation, and spec v1 was wrong to assert
it. See [SPEC.md §11](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md).

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
| `Arxid.fromKeyBytes` | `(bytes: Uint8Array \| readonly number[]) => Arxid` (8 bytes, big-endian) |
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

Full threat model:
[SPEC.md §9](https://github.com/lucasolopes/arxid/blob/main/spec/SPEC.md).

**Recovering an id from a code without the key is not computationally
infeasible.** arxid raises enumeration from "count upward" to "do some work". It
is **not** encryption, **not** a MAC (no authentication, no integrity), and has
had **no independent cryptographic audit**. Any well-formed 7-character code
decodes to *some* id, so a code decoding successfully does not mean you issued
it. **ID obfuscation is not access control.**

Operationally: random key per deployment, loaded from the environment or a
secret manager and never committed; a **separate key per resource type**, since
there is no tweak and one key is one global mapping; and do not echo internal
ids back in responses or errors.

This package implements **spec v2**. Spec v1 (0.1.x) is withdrawn — it used 4
rounds, which review showed to be separable from a random permutation at ~2^13
chosen queries. Codes issued by 0.1.x do not decode here.

## Development

```sh
npm install
npm run typecheck
npm test        # includes the canonical vectors
npm run build
```

## License

MIT. See [`LICENSE`](https://github.com/lucasolopes/arxid/blob/main/LICENSE).
