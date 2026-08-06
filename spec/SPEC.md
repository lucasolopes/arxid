# arxid — Specification v2 (normative)

`arxid` is a **keyed, reversible permutation for obfuscating sequential integer
IDs**, so they can be exposed publicly (URLs, public API responses) without being
trivially guessable, while remaining recoverable to the original ID with the same
key.

It is a **balanced Feistel network with an ARX (Add-Rotate-XOR) round function**,
format-preserving over a 40-bit domain, plus an optional fixed-length base62
string encoding.

This document is **normative**. Any implementation in any language that conforms
to it MUST produce byte-identical outputs for identical `(key, id)` inputs. A
conforming implementation is verified against the canonical test vectors in
`/vectors/vectors.json`. **A port that passes the round-trip but produces
different outputs than this spec is non-conforming** — round-trip is symmetric
and hides width/wrapping bugs; only the known-answer vectors catch them.

Behavioral change = new spec version, never a silent bump.

> **Spec v1 is withdrawn.** It was published as "frozen" without any public
> review period, and review found two measurable defects: a chosen-query
> distinguisher at ~2^13 queries, and an excess of consecutive ids mapping to
> adjacent codes. Both are fixed in v2. **Codes issued under v1 do not decode
> under v2.** See §11 for what changed and the measurements behind it.
>
> v2 is **not** frozen against further review. If you find a problem with it,
> that is a reason to publish v3, not a reason to defend v2.

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

### 2.1 Key storage and byte order

A `u64` has no endianness, but a key **at rest** does. The moment it lives in an
environment variable, a config file, or a KMS blob, both ends must agree on byte
order, or two services sharing a key will compute different permutations.

- Keys serialized as 8 raw bytes MUST use **big-endian** order.
- Implementations SHOULD offer a byte-oriented constructor alongside the integer
  one (`from_key_bytes` in Rust, `Arxid.fromKeyBytes` in TypeScript) so callers
  are not left to pick an order themselves.
- Keys written as hex or decimal text carry no ambiguity and need no convention.

### 2.2 One key is one global mapping (no tweak)

arxid takes no tweak, nonce, or context parameter. A key therefore defines
exactly one mapping over the whole domain, and the same id yields the same code
everywhere that key is used.

If `orders` and `users` share a key, then `/orders/5kusgvr` and `/users/5kusgvr`
refer to the same underlying id, and that is observable by anyone. **Derive a
separate key per resource type** when the linkage matters. Deriving them from
one root secret (`HKDF(root, "orders")`, or any KDF you already trust) is fine;
what is not fine is reusing one key and assuming the contexts are independent.

Spec v1 did not document this at all.

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

`ROUNDS = 6`. Part of the contract — changing it changes every output.

**Why 6, and why v1's 4 was wrong (informative).** Spec v1 chose 4 because it
was the smallest count reaching 0.5000 *mean* avalanche with full 40/40 bit
coverage. That was the wrong criterion: mean avalanche is a necessary condition
that is cheap to satisfy and evidences almost nothing. Three independent
measurements all place the real threshold at 5:

| Measurement | 4 rounds | 5 rounds | 6 rounds | Reproduce with |
|---|---|---|---|---|
| Worst individual (input bit, output bit) pair, deviation from 0.5 | 0.1117 | 0.0039 | 0.0041 | `examples/avalanche.rs` |
| Chosen-query distinguisher vs. a random permutation | separates at ~2^13 queries, ratio 1.85 by 2^16 | no bias, ratio ~1.00 | no bias out to the 2^20 slice, ratio ~1.00 | `examples/distinguisher.rs` |
| Consecutive ids mapping to adjacent codes | hundreds to thousands× the rate of chance | at the rate of chance | at the rate of chance | `examples/adjacency.rs` |

The distinguisher and the adjacency excess are both structural to 4 rounds: they
persist under v2's revised key schedule, so neither was an artifact of the fold
in §4. Note also that the first row was already visible in v1's own published
sweep — the signal was in the table and was not acted on.

Five rounds closes all three; arxid uses **six**. The sixth round buys no
measurable improvement over the fifth — it is there because Patarin's generic
attacks on Feistel schemes recommend at least six rounds for a pseudorandom
permutation (§9.4), and the cost of taking that recommendation is lost in the
base62 step. None of this is a cryptographic security margin (see §9): five is
the point at which the measurable structure the authors know how to look for
disappears, and six is that plus the literature's recommended minimum.

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
// all arithmetic on u64 with wraparound (mod 2^64), except the final fold
GOLDEN = 0x9E3779B97F4A7C15                 // u64 constant (golden ratio)

x = rotl64(key, (round * 7 + 1) mod 64)
    XOR ( (GOLDEN * (round + 1)) mod 2^64 )   // wrapping multiply, u64

subkey = ( low32(x) + high32(x) ) mod 2^32    // wrapping ADD, u32 -> u32
```

Notes:
- `rotl64(v, n)` = circular left rotation of a **64-bit** value by `n` bits.
- The rotation amount `round * 7 + 1` for rounds 0..4 is `1, 8, 15, 22, 29` —
  all `< 64`, so no reduction is observable, but implementations SHOULD reduce
  `mod 64` for safety/generality.
- The multiply `GOLDEN * (round + 1)` is a **64-bit wrapping** multiply.
- `high32(x)` is `x >> 32` with a **logical** (unsigned) right shift.
- The final fold is a **wrapping add**, not a XOR.

### 4.1 Why the fold is an add (informative)

Spec v1 folded with `low32(x) XOR high32(x)`, and that made the schedule
non-injective in a specific, avoidable way. Complementing the key complements
`x`, and `lo(!x) XOR hi(!x) = !lo(x) XOR !hi(x) = lo(x) XOR hi(x)` — the
complement cancels. So `key` and `!key` derived identical subkeys and defined
the *same* permutation, for every key, and the effective key space was 2^63.

v1 documented this and declared it frozen behavior rather than a defect. It was
a defect: it was discovered after the fact rather than designed, which is the
part that matters. Addition does not commute with complement, so v2 recovers the
full 2^64 key space with one operator change. The canonical vectors now include
`u64::MAX`, which v1 had to exclude because its rows would have duplicated those
of key `0`.

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

## 9. Threat model and security posture

Spec v1 described what arxid *is not* and never stated what it defends against.
This section states it positively. Read the whole thing before deploying.

### 9.1 The short answer

**Is it computationally infeasible to recover an id from a code without the
key? No.** arxid raises the cost of enumeration from "count upward" to
"something you have to actually work at". It does not put it out of reach. If
your answer to "what happens when an attacker reverses this" is anything worse
than "they learn a row ordering I would rather they did not", arxid is the wrong
tool — see §9.5.

### 9.2 What it defends against

| Adversary | Outcome |
|---|---|
| Walks `/orders/1`, `/orders/2`, … to enumerate your rows | **Defeated.** Codes carry no ordering, and only 1 in ~2^40 random 7-character strings is even in range. |
| Reads your row count or growth rate off sequential URLs | **Defeated.** Codes are unordered, so `id → code` leaks neither position nor volume. |
| Scrapes ids seen in one place to guess ids elsewhere | **Defeated only if you use a separate key per resource type** (§2.2). |

### 9.3 What it does not defend against

| Adversary | Outcome |
|---|---|
| Wants to forge or tamper with a code | **Not defended.** No authentication, no integrity, not a MAC. Every well-formed in-range code decodes to *some* id; decoding successfully proves nothing about origin. Layer a real MAC if you need unforgeability. |
| Can submit chosen codes and observe the decoded id | **Structure is measurable.** See §9.4. |
| Has the key | Total break, by construction. It is a reversible permutation. |
| Is stopped only by not knowing the id | **Not defended.** Id obfuscation is not authorization. Never make a code the sole barrier to a resource. |

### 9.4 Known limits, with numbers

- **Small domain.** 40 bits is small by cryptographic standards. It is a
  format-preserving choice driven by the 7-character code length, not a security
  parameter.
- **Small halves.** The Feistel halves are 20 bits. The Luby–Rackoff results
  that motivate Feistel constructions only guarantee security to the birthday
  bound in the half width — about 2^10 queries here — and they do not apply to
  this construction anyway, because the ARX round function is not a PRF and the
  subkeys are derived rather than independent. Treat 2^10 as a ceiling on what
  could be *proved*, not as an attack.
- **Measured distinguisher.** For spec v1 (4 rounds), a chosen-query
  distinguisher separates the permutation from a random one at about **2^13
  queries** and reaches ratio 1.85 by 2^16. Spec v2 (6 rounds) shows no bias out
  to the full **2^20** slice — the largest an attacker can submit with the high
  half fixed, so this is the ceiling of the attack, not an arbitrary cutoff.
  Reproduce with `examples/distinguisher.rs`.
- **Query reachability.** Encode-side, that attack needs many ids sharing their
  low 20 bits, which a deployment with ids under a million cannot supply at all.
  **Decode-side it is free**: the attacker picks codes. It requires only that
  the application reveal the decoded id for a submitted code — so *do not echo
  internal ids back*. This is the practical reason id leakage matters.
- **Round count meets the literature's recommendation, which is not a proof.**
  5 is where the structure the authors know how to measure disappears (§3.2);
  arxid uses 6, the **minimum Patarin's generic attacks on Feistel schemes
  recommend** for a pseudorandom permutation. Meeting that floor is not a
  security proof for *this* construction: the round function is not a PRF and the
  subkeys are derived rather than independent, so the Luby–Rackoff and Patarin
  analyses do not transfer directly. It is the recommended minimum, honestly met,
  and nothing more.
- **No independent audit.** This construction has had no external cryptographic
  review. The round function has no published lineage or differential/linear
  analysis. Non-enumerability is a measured statistical property, not a proof.

### 9.5 When to use something else

| You need | Use |
|---|---|
| Ids that are genuinely confidential | Random ids or UUIDv7 with an indexed column. Nothing derived from the sequence can beat not having a sequence. |
| Format-preserving encryption with real analysis behind it | **FF1 or FF3-1** (NIST SP 800-38G) with AES. |
| Public ids that cannot be forged | HMAC over the id, stored in a lookup column. |
| Several values packed into one short id | [Sqids](https://sqids.org). |

### 9.6 Operational requirements

- Use a random key per deployment, loaded from the environment or a secret
  manager, never committed. Do not copy a key out of any documentation,
  including this repository's.
- Use a separate key per resource type (§2.2).
- Store keys as 8 big-endian bytes, or as text (§2.1).
- Do not return internal ids in responses or errors (§9.4).

---

## 10. Versioning

- Every parameter above (width, rounds, the ARX constants `7/13/17`, the golden
  constant, the schedule and its fold, the alphabet and its order, code length)
  is part of the contract for a given spec version.
- Any observable behavior change requires a **new spec version** and a
  correspondingly new set of vectors. Never a silent change.
- A port producing output different from the reference / vectors is a **bug in
  the port**, not an acceptable variation.
- Implementations MUST expose the spec version they implement (`SPEC_VERSION`).
- A spec version being current is **not** a claim that it is final. v1 was
  published as frozen before anyone had reviewed it, which is what this project
  got wrong the first time. Versions exist so the algorithm can change when
  review finds something; they are not a commitment that it will not.

## 11. What changed in v2, and why

v1 was released as "frozen at spec v1" with no public review period. Review then
found two measurable defects. Both are fixed here, and the vectors are
regenerated: **codes issued under v1 do not decode under v2.**

| # | v1 | v2 | Why |
|---|---|---|---|
| 1 | `ROUNDS = 4` | `ROUNDS = 6` | 4 rounds is separable from a random permutation at ~2^13 chosen queries, and maps consecutive ids to adjacent codes hundreds to thousands of times more often than chance. Both measure clean by 5; v2 uses 6 to meet Patarin's recommended minimum. See §3.2. |
| 2 | subkey fold `lo XOR hi` | subkey fold `lo + hi` | XOR cancelled under key complement, so `key` and `!key` were the same permutation and the key space was 2^63. See §4.1. |

Two documentation claims were also **withdrawn as false**:

- *"Consecutive ids do not produce consecutive codes."* This is not a property
  of a good permutation — an ideal one produces such pairs about twice per full
  domain, and guaranteeing zero would itself be a distinguisher. v1 asserted it
  in three READMEs and pinned it in three test files. Worse, v1's construction
  produced such pairs far *more* often than chance. A concrete v1 counterexample:
  under key `0x652BFD48C7ED0458`, ids 48326508 and 48326509 encode to `5kusgvr`
  and `5kusgvs`.
- *"The effective key space is 2^63 ... frozen behavior, not a defect."* It was
  a defect. See §4.1.

Neither change was needed for correctness — v1 round-tripped correctly and
always had. They were needed because v1 claimed properties it did not have.
