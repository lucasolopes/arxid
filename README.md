# arxid

[![CI](https://github.com/lucasolopes/arxid/actions/workflows/ci.yml/badge.svg)](https://github.com/lucasolopes/arxid/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Spec v2](https://img.shields.io/badge/spec-v2-informational)](spec/SPEC.md)

**Sequential id in, 7-character unordered code out. Reversible with the key.
Identical in every language.**

```rust
let arxid = Arxid::new(key);

arxid.obfuscate_str(1000);   // "BmgnvEe"
arxid.obfuscate_str(1001);   // "4ndJOI8"
arxid.obfuscate_str(1002);   // "FgxNwC0"

arxid.deobfuscate_str("4ndJOI8");   // Some(1001)
```

No lookup table, no extra column, no state. Just a keyed permutation over the
integers you already have.

> **Obfuscation, not encryption.** arxid makes ids non-sequential; it does not
> make them secret. It is not a MAC, not access control, and has had no
> independent cryptographic review. Read the
> [threat model](spec/SPEC.md#9-threat-model-and-security-posture) before
> deploying — it says plainly what this does and does not stop.

> **Spec v1 is withdrawn.** It shipped "frozen" before anyone reviewed it, and
> review found two real defects. v2 fixes both. **Codes issued by 0.1.x do not
> decode under 0.2.** See [what changed](spec/SPEC.md#11-what-changed-in-v2-and-why).

## Install

<table>
<tr><th>Rust</th><th>TypeScript</th></tr>
<tr valign="top"><td>

```toml
arxid = "0.2"
```

```rust
use arxid::Arxid;

// Load your own random key. Never hardcode one.
let a = Arxid::new(std::env::var("ARXID_KEY")?.parse()?);

let code = a.obfuscate(1001);
a.deobfuscate(code);          // 1001

let s = a.obfuscate_str(1001);
a.deobfuscate_str(&s);        // Some(1001)
```

</td><td>

```sh
npm install arxid
```

```ts
import { Arxid } from "arxid";

// Load your own random key. Never hardcode one.
const a = new Arxid(BigInt(process.env.ARXID_KEY!));

const code = a.obfuscate(1001);
a.deobfuscate(code);          // 1001

const s = a.obfuscateStr(1001);
a.deobfuscateStr(s);          // 1001
```

</td></tr>
</table>

## The problem

Exposing `/orders/1042` tells everyone that orders `1041` and `1043` exist. Your
row count, your growth rate, and your competitors' ability to scrape you all
leak from the URL. Random UUIDs plug the leak but cost you a compact primary key
and a compact URL, and they need a second indexed column.

arxid keeps the sequential integer in your database and shows the world a
7-character code that reveals nothing about ordering or volume.

## How it works

A balanced Feistel network with an ARX (Add-Rotate-XOR) round function,
format-preserving over a 40-bit domain, plus an optional base62 encoding.

| | |
|---|---|
| Domain | `[0, 2^40 - 1]` = `[0, 1099511627775]` (about 1.1 trillion ids) |
| Key | `u64`, full 2^64 key space |
| Rounds | 5 |
| Output | base62, fixed 7 characters, alphabet `0-9A-Za-z` in that order |
| Tweak | none — one key is one global mapping, so use a key per resource type ([§2.2](spec/SPEC.md)) |
| Dependencies | none in the core (pure integer arithmetic) |

40 bits encodes to exactly 7 base62 characters and fits inside IEEE-754's safe
integer range, so no language needs a big-integer type for the public values.

### Why 6 rounds

Spec v1 used 4, chosen as the smallest count reaching 0.5000 *mean* avalanche.
That was the wrong criterion — mean avalanche is cheap to satisfy and evidences
almost nothing. Three independent measurements, each reproducible from this repo,
put the real threshold at 5:

| Measurement | 4 rounds (v1) | 5 rounds | 6 rounds (v2) | Reproduce |
|---|---:|---:|---:|---|
| Worst (input bit, output bit) pair, deviation from ideal 0.5 | 0.1117 | 0.0039 | **0.0041** | [`examples/avalanche.rs`](impl/rust/examples/avalanche.rs) |
| Chosen-query distinguisher vs. a random permutation | separates at ~2^13 queries, ratio 1.85 by 2^16 | ratio ~1.00 | **no bias to the 2^20 wall**, ratio ~1.00 | [`examples/distinguisher.rs`](impl/rust/examples/distinguisher.rs) |
| Consecutive ids landing on adjacent codes | hundreds to thousands × the rate of chance | at the rate of chance | **at the rate of chance** | [`examples/adjacency.rs`](impl/rust/examples/adjacency.rs) |

Five rounds already clears all three, and the worst-pair and distinguisher
columns are flat from there. arxid ships **6** anyway: Patarin's generic attacks
on Feistel schemes recommend at least six rounds for a pseudorandom permutation,
and the round beyond the measured threshold is lost in the noise of the base62
step (see below), so there is no reason to argue with the recommendation instead
of taking it.

The worst-pair row was already in v1's own published table. The signal was there
and was not acted on. The other two came out of public review after release,
which is the review v1 should have had before being declared frozen.

**None of this is a security proof.** It is the point where the structure we know
how to measure disappears, plus a round of margin. See the
[threat model](spec/SPEC.md#9-threat-model-and-security-posture).

### Performance

The structural claim is the durable one:
[`benches/compare_bench.rs`](impl/rust/benches/compare_bench.rs) holds the
network constant (same balanced Feistel, same round count, same 40-bit width,
same base62 step) and swaps **only** the round function, ARX for HMAC-SHA256.
That isolates one thing: doing integer arithmetic instead of hash calls is
**about an order of magnitude faster**, and that gap survives whatever CPU you
are on.

The absolute numbers do not travel, and this repo has now demonstrated that on
itself. Measured with criterion on a 13th Gen Intel Core i7-1355U, 16 GB RAM,
Windows 11 Pro:

| Benchmark | Time/op |
|---|---:|
| `permute/obfuscate` (raw u64 → u64) | ~15 ns |
| `permute/deobfuscate` | ~18 ns |
| `arxid/encode` (permute + base62 String) | ~127 ns |
| `feistel_hmac/encode` (same Feistel, HMAC-SHA256 round) | ~1265 ns |

**Treat these as order-of-magnitude only.** Re-running the identical code path
on this machine across sessions produced figures spanning 3x, and the ARX/HMAC
ratio ranged from 10x to 22x depending on nothing but how busy the laptop was.
If the number matters to you, measure it on your own hardware with `cargo
bench`; do not trust the table above, including the version of it that used to
quote 4.42 ns to three significant figures.

One honest oddity: `deobfuscate` benchmarks consistently slower than
`obfuscate` even though a Feistel does identical work in both directions. That
is a codegen artifact of the reversed loop, not an algorithmic asymmetry.

### What the extra rounds cost

Going from v1's 4 rounds to v2's 6 is not free, but where the cost lands is the
whole story. Measured by interleaving every arm in one process and taking the
minimum over 40 repetitions, so session-to-session drift cancels instead of
swamping the comparison
([`examples/roundcost.rs`](impl/rust/examples/roundcost.rs)):

- **On the bare permutation** the cost is real and roughly linear: each ARX round
  adds on the order of ~1.5 ns, so the two extra rounds are about +3 ns — a large
  fraction of a permutation that only takes a handful of nanoseconds to begin
  with.
- **On the path an application actually pays** (permutation + the base62
  `String`) it disappears. The allocation costs roughly ten times the whole
  permutation, and its own run-to-run jitter is several nanoseconds — larger than
  the round cost it would be hiding. One run of this very example clocked the
  5-round encode as *faster* than the 4-round encode, which is impossible and is
  exactly the point: on that path the round count is below the noise floor.

So the sixth round costs a couple of nanoseconds on a number nobody ships and
nothing measurable on the number they do — which is also why "measure avalanche
and pick the cheapest count" was never a real performance argument to begin with.
Don't trust these figures to three significant figures; measure on your own
hardware with `cargo bench`.

## Portability

The product here is not a library in one language. It is a **versioned
specification plus canonical test vectors**:

- [`spec/SPEC.md`](spec/SPEC.md) — normative, currently v2. Width, rounds, the
  ARX constants, the golden constant, the key schedule, the alphabet and its
  order, the code length: all contract, per version.
- [`vectors/vectors.json`](vectors/vectors.json) — 71 known-answer tests,
  generated by the reference implementation. Every port validates against this
  exact file.

An id obfuscated in Rust deobfuscates identically in TypeScript because both
agree with the vectors. **A port that disagrees with the vectors is a bug in the
port**, not a variation.

Round-trip tests alone are not enough: they are symmetric and pass happily on an
implementation that wraps at the wrong width or truncates the 64-bit key
schedule. Only known-answer vectors catch that. See
[`vectors/README.md`](vectors/README.md).

### Ports

| Language | Status | Location |
|---|---|---|
| Rust | ✅ reference implementation | [`impl/rust`](impl/rust) |
| TypeScript / JavaScript | ✅ native port | [`impl/ts`](impl/ts) |
| Go, Python, C#, Kotlin, Ruby, PHP | planned | — |

Adding one is the most useful contribution you can make: reimplement the spec
natively, pass the vectors, open a PR. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

### WASM is not the portability story

The Rust crate has an optional `wasm` feature so you can run the reference
implementation itself in a browser. **It is not how arxid travels between
languages.** Interoperability comes from native ports validated against the
vectors, not from shipping one binary everywhere. Prefer the native port.

## Compared to Sqids (and Hashids)

[Sqids](https://sqids.org) is the obvious alternative and is a fine tool. It
solves a broader problem; arxid solves a narrower one more strictly.

| | arxid | Sqids |
|---|---|---|
| Output length | fixed 7 chars | variable (minimum configurable, no maximum) |
| Values per code | one | one or many |
| Keying | `u64` key driving an ARX key schedule | shuffled alphabet |
| Domain | `[0, 2^40-1]` | unbounded |
| Profanity blocklist | no | yes |
| Ecosystem | 2 implementations | many, mature |
| Interop guarantee | canonical vectors, versioned spec | per-implementation |
| Track record | released 2026, one revision after first review | years of production use |

**Choose Sqids** if you need to pack several numbers into one id, have ids
beyond 2^40, want the profanity blocklist, or want a library that already
exists in your language today.

**Choose arxid** if you need every code to be exactly the same length, and you
want the obfuscation driven by an actual key with measured diffusion rather than
by an alphabet permutation.

**Choose Sqids** also if maturity matters more to you than any of the above.
arxid is new, and it has already had to withdraw a spec version.

**Do not choose either as a security control.** Sqids is explicit about this:
*"There is no encryption of any kind"* and *"Given enough effort, somebody could
reverse-engineer your shuffled alphabet, so this is by no means a technique to
hide sensitive data"*
([Sqids FAQ](https://sqids.org/faq)). arxid uses a real key schedule instead of
an alphabet shuffle, but it is equally **not encryption, not a MAC, and not
independently audited**. Neither library is a substitute for authorization.

## Security

The full threat model is
[SPEC.md §9](spec/SPEC.md#9-threat-model-and-security-posture). The summary:

**Is it computationally infeasible to recover an id from a code without the key?
No.** arxid raises enumeration from "count upward" to "actually do some work".
It does not put it out of reach.

**It stops:** walking `/orders/1`, `/orders/2`, …; reading your row count or
growth rate off sequential URLs.

**It does not stop:** forgery or tampering (no authentication, no integrity, not
a MAC — every well-formed in-range code decodes to *some* id); anyone holding
the key; anyone your authorization layer should have stopped. **Id obfuscation
is not access control.**

**Known limits, with numbers.** The 40-bit domain is small by cryptographic
standards, and the 20-bit Feistel halves are smaller. Luby–Rackoff only
guarantees security to the birthday bound in the half width — ~2^10 queries —
and does not apply here anyway, since the ARX round function is not a PRF.
Spec v1's 4-round construction was separable from a random permutation at ~2^13
chosen queries; that structure is gone by 5 rounds and v2's 6 rounds shows no
bias out to the full 2^20 slice an attacker can reach. Even so, this is where
*measurable* structure ends, not a proven margin — Patarin's generic Feistel
attacks recommend at least 6 rounds for a pseudorandom permutation, which is the
bar v2 now meets and does not claim to exceed. The round function has no
published lineage or differential analysis, and there has been **no independent
cryptographic audit**.

**Operational requirements.** Random key per deployment, from the environment or
a secret manager, never committed and never copied out of documentation. A
**separate key per resource type** — there is no tweak, so one key is one global
mapping and `/orders/{code}` and `/users/{code}` would otherwise be linkable.
Keys stored as bytes are big-endian. And **do not echo internal ids back**: the
one reachable form of the distinguisher needs exactly that.

**If you need more than this**, use random ids or UUIDv7 (confidentiality), FF1
or FF3-1 from NIST SP 800-38G (real format-preserving encryption), or an HMAC in
a lookup column (unforgeable public ids).

To report a vulnerability, see [`SECURITY.md`](SECURITY.md).

## Versioning

SemVer, per implementation. The **algorithm** is versioned separately, by spec
version, currently **v2**. Implementations expose it as `SPEC_VERSION`.

Any observable behavior change requires a **new spec version and a new set of
vectors**, never a silent bump. A port producing output different from the
reference is a bug in the port.

A current spec version is not a promise that it is final. v1 was published as
"frozen" before anyone had reviewed it; that was the mistake, and the version
mechanism exists precisely so the next one can be corrected too.

## License

MIT. See [`LICENSE`](LICENSE).
