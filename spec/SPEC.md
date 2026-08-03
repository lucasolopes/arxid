# arxid — Specification v1 (normative, frozen)

`arxid` is a **keyed, reversible permutation for obfuscating sequential integer
IDs**, so they can be exposed publicly (URLs, public API responses) without being
enumerable, while remaining recoverable to the original ID with the same key.

It is a **balanced Feistel network with an ARX (Add-Rotate-XOR) round function**,
format-preserving over a 40-bit domain, plus an optional fixed-length base62
string encoding.

This document is **normative and frozen for spec v1**. Any implementation in any
language that conforms to this document MUST produce byte-identical outputs for
identical `(key, id)` inputs. A conforming implementation is verified against the
canonical test vectors in `/vectors/vectors.json`. **A port that passes the
round-trip but produces different outputs than this spec is non-conforming** —
round-trip is symmetric and hides width/wrapping bugs; only the known-answer
vectors catch them.

Behavioral change = new spec version, never a silent bump.

---

## 1. Domain

- Width: **`WIDTH_BITS = 40`**.
- `MAX_ID = 2^40 - 1 = 1_099_511_627_775`.
- Valid input domain: integers in `[0, MAX_ID]`.
- Inputs outside the domain are **reduced modulo 2^40** (bitwise `& MAX_ID`)
  before processing. The functions are **total and never error/panic** on any
  integer input. (String decode is stricter — see §6.)

40 bits fits inside IEEE-754 double's safe integer range (2^53), so the **public
values** need no big-integer type. The **key schedule operates on 64 bits**,
which does require care in languages without native 64-bit unsigned integers
(see §7).

---

## 2. Key

- The key is a **`u64`** (unsigned 64-bit integer).
- The full 64-bit value participates in the key schedule (§4). Implementations
  MUST NOT truncate the key to 32 bits.

### 2.1 Key complement equivalence (informative)

The key schedule of §4 is **not injective**: `key` and `!key` (bitwise
complement) derive the *same* subkeys and therefore define the *same*
permutation, for every key.

Why: `subkey` ends in `low32(x XOR (x >> 32))`, which is `lo(x) XOR hi(x)`.
Replacing `key` with `!key` gives `rotl64(!key, n) = !rotl64(key, n)`, so `x`
becomes `!x`, and `lo(!x) XOR hi(!x) = !lo(x) XOR !hi(x) = lo(x) XOR hi(x)`.
The complement cancels.

Consequences:

- The **effective key space is 2^63, not 2^64**. Keys come in equivalent pairs
  `{k, !k}`. This costs one bit of key search; it does not affect the
  permutation's format-preserving or round-trip properties.
- This is **frozen behavior**, part of spec v1, not a defect to be repaired. A
  port that "fixes" it produces different outputs and is non-conforming. The
  canonical vectors do not include `u64::MAX`, precisely because its outputs
  would be identical to those of key `0` and would test nothing new.
- Random 64-bit keys remain fine in practice. Do not deliberately pick a key as
  the complement of another and expect a different permutation.

---

## 3. Feistel structure

Let `half = WIDTH_BITS / 2 = 20` and `half_mask = 2^20 - 1 = 0xFFFFF`.

### 3.1 Split (input → L, R)

Given a 40-bit input `x` (already reduced to `[0, MAX_ID]`):

```
L = (x >> 20) & 0xFFFFF     // high 20 bits
R =  x        & 0xFFFFF     // low  20 bits
```

`L` and `R` are each **20-bit values held in an unsigned 32-bit register** (the
round function operates in u32; results are masked back to 20 bits).

### 3.2 Rounds

`ROUNDS = 4`. (Calibrated: 4 is the smallest round count reaching exact 0.5000
avalanche with full 40/40 bit-coverage. Frozen — do not change; changing it
changes every output.)

**Note on what the calibration measured (informative).** The 0.5000 figure is
the *mean* fraction of output bits flipped, and the 40/40 figure is structural
coverage. Neither is per-pair balance. Measured over 50,000 sampled ids, the
worst individual (input bit, output bit) pair deviates from the ideal 0.5 by
about **0.166 at 4 rounds**, falling to about 0.008 at 5 rounds. So 4 rounds
closes diffusion on average and structurally, but strict per-pair SAC is not
reached until 5. This is a documented property of spec v1, not a defect to
repair: changing the round count changes every output. Treat the round count as
a calibrated diffusion target, not a security margin (see §9). The measurement
is reproducible via `impl/rust/examples/avalanche.rs`.

**Forward (encode)** — for `round = 0, 1, 2, 3` in ascending order:

```
f      = round_fn(R, key, round, half=20)
new_L  = R
new_R  = L XOR f
L, R   = new_L, new_R
```

**Inverse (decode)** — for `round = 3, 2, 1, 0` in descending order:

```
R_prev = L
f      = round_fn(R_prev, key, round, half=20)
L_prev = R XOR f
L, R   = L_prev, R_prev
```

### 3.3 Recombine (L, R → output)

```
output = (L << 20) | R
```

Output is guaranteed in `[0, MAX_ID]`.

---

## 4. Key schedule — `subkey(key, round) -> u32`

Derives the 32-bit subkey for a given round from the 64-bit master key.

```
// all arithmetic on u64 with wraparound (mod 2^64), except final cast
GOLDEN = 0x9E3779B97F4A7C15                 // u64 constant (golden ratio)

x = rotl64(key, (round * 7 + 1) mod 64)
    XOR ( (GOLDEN * (round + 1)) mod 2^64 )   // wrapping multiply, u64

subkey = (x XOR (x >> 32)) truncated to lower 32 bits   // -> u32
```

Notes:
- `rotl64(v, n)` = circular left rotation of a **64-bit** value by `n` bits.
- The rotation amount `round * 7 + 1` for rounds 0..3 is `1, 8, 15, 22` — all
  `< 64`, so no reduction is observable, but implementations SHOULD reduce
  `mod 64` for safety/generality.
- The multiply `GOLDEN * (round + 1)` is a **64-bit wrapping** multiply.
- `x >> 32` is a **logical** (unsigned) right shift.

---

## 5. Round function — `round_fn(r, key, round, half=20) -> u32`

ARX mixing of one 20-bit half with the round subkey. **All arithmetic is on
`u32` with wraparound (mod 2^32)**; the result is masked to 20 bits.

```
half_mask = 2^20 - 1 = 0xFFFFF          // for half = 20
rk = subkey(key, round)                 // u32, from §4

x = (r + rk)          mod 2^32          // wrapping add, u32
x = x XOR rotl32(x, 7)
x = (x + rotl32(x, 13)) mod 2^32        // wrapping add, u32
x = x XOR rotl32(x, 17)
return x AND half_mask                  // -> 20-bit result in a u32
```

- `rotl32(v, n)` = circular left rotation of a **32-bit** value by `n` bits.
- Every `+` is a **32-bit wrapping** add. Every XOR/rotate is over 32 bits.
- The mask to 20 bits happens **only at the end**; intermediate steps use the
  full 32-bit width. This matters: masking earlier changes the result.

---

## 6. Core API (language-agnostic contract)

```
obfuscate(id: u64, key: u64) -> u64        // encode: Feistel forward, §3.2 forward
deobfuscate(code: u64, key: u64) -> u64    // decode: Feistel inverse, §3.2 inverse
```

- `id` and `code` are reduced `& MAX_ID` on entry.
- `deobfuscate(obfuscate(x, k), k) == (x & MAX_ID)` for all `x`, all `k`.
- Total, never panics/throws on integer input.

(Implementations SHOULD name these `obfuscate`/`deobfuscate` rather than
`encode`/`decode`, to avoid colliding with the string-encoding layer in §7. The
reference implementation uses `obfuscate`/`deobfuscate` throughout, internally
and publicly.)

---

## 7. String encoding (optional layer)

Serializes a 40-bit integer to a **fixed 7-character base62 string** and back.

### 7.1 Alphabet

```
0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz
```

Index 0..61 in this exact order: digits `0-9` (0..9), uppercase `A-Z` (10..35),
lowercase `a-z` (36..61). This order is **normative** — it is not the same as
some other base62 orderings. Do not reorder.

### 7.2 Encode — `to_base62(n: u64) -> String`

- `CODE_LEN = 7`.
- Big-endian base-62: most significant digit first.
- Left-padded with `'0'` (alphabet index 0) to exactly 7 characters.
- `n = 0` encodes to `"0000000"`.
- Values `n > MAX_ID` are **silently truncated** by the caller's domain reduction;
  `to_base62` itself writes at most 7 digits (higher digits are dropped). Callers
  MUST reduce to the domain first.

Algorithm (matches reference):

```
buf = ['0'; 7]
i = 7
while n > 0 and i > 0:
    i -= 1
    buf[i] = ALPHABET[n mod 62]
    n = n / 62            // integer division
return utf8(buf)
```

### 7.3 Decode — `from_base62(s) -> Option<u64>`

Returns "no value" (None/null/error, per language idiom) — NOT a panic — when:
- `s` length != 7, **or**
- any character is not in the alphabet, **or**
- the decoded value `> MAX_ID`.

Algorithm:

```
if len(s) != 7: return None
n = 0
for each char c in s (left to right):
    d = index_of(c) in alphabet, else return None
    n = n * 62 + d          // reject on overflow if using fixed-width ints
if n <= MAX_ID: return n else return None
```

- `val(c)`: `'0'..'9' -> 0..9`, `'A'..'Z' -> 10..35`, `'a'..'z' -> 36..61`,
  else invalid.
- The reference uses checked multiply/add; a port in a language with 64-bit ints
  is safe against overflow for 7 base62 digits (max `62^7 ≈ 3.5e12 < 2^42`), but
  MUST still enforce the `<= MAX_ID` bound.

### 7.4 String-level convenience API

```
obfuscate_str(id: u64, key: u64) -> String        = to_base62(obfuscate(id, key))
deobfuscate_str(s, key: u64) -> Option<u64>        = from_base62(s).map(|c| deobfuscate(c, key))
```

---

## 8. Cross-language implementation hazards (informative)

These are the exact places a port silently diverges. The vectors exist to catch
them; implementers should read this before porting.

- **u32 wrapping.** Every `+` in §4/§5 is modulo 2^32 (or 2^64 in the schedule).
  Languages with arbitrary-precision ints (Python) MUST mask with `& 0xFFFFFFFF`
  (or `& 0xFFFFFFFFFFFFFFFF`) after each add/multiply/rotate.
- **Rotation width.** `rotl32` rotates in 32 bits; `rotl64` in 64 bits. Do not
  mix. `rotl32(v, n) = ((v << n) | (v >> (32 - n))) & 0xFFFFFFFF`.
- **Logical vs arithmetic shift.** `x >> 32` in §4 is unsigned/logical.
- **JavaScript / TypeScript specifically:**
  - Public domain (40-bit) fits in `number`, but the **key schedule needs
    `BigInt`** (64-bit multiply + 64-bit rotate + the `0x9E3779B97F4A7C15`
    constant). Compute `subkey` in BigInt, then narrow to a 32-bit `number`.
  - Bitwise operators coerce to **signed int32**; force unsigned with `>>> 0`
    after 32-bit ops, and never use `<<`/`>>` for the 64-bit parts.
- **Go / Kotlin / Rust / C#:** have native fixed-width unsigned types
  (`uint32`/`uint64`, `UInt`/`ULong`) and rotate helpers; near-direct
  transcription. Use the unsigned types, not the signed ones.

---

## 9. Security posture (normative for docs, informative for behavior)

- arxid is a **keyed reversible permutation for ID obfuscation** — defeating
  trivial enumeration of sequential resources. It is **not** encryption of
  arbitrary data.
- It provides **no authentication and no integrity**: it is **not a MAC**. A
  valid-looking output does not prove it was produced by a holder of the key. For
  unforgeability, layer a real MAC on top.
- Non-enumerability is a **measured statistical property** (avalanche/SAC over a
  reduced-round ARX permutation), **not** a cryptographic guarantee. Resistance
  to key recovery depends on the round count and construction. This construction
  has **not** undergone independent cryptographic audit.
- The **effective key space is 2^63**, because `key` and `!key` define the same
  permutation (§2.1). Budget key strength accordingly.
- **ID obfuscation is not access control.** Do not use arxid as the sole
  authorization barrier for a resource that must stay secret.
- Each deployment SHOULD use its own random key, kept out of source control.

---

## 10. Versioning

- The algorithm is **frozen at spec v1**. Every parameter above (width, rounds,
  the ARX constants `7/13/17`, the golden constant, the schedule, the alphabet
  and its order, code length) is part of the contract.
- Any observable behavior change requires a **new spec version** and a
  correspondingly new set of vectors. Never a silent change.
- A port producing output different from the reference / vectors is a **bug in
  the port**, not an acceptable variation.
