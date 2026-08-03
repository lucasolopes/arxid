# Contributing to arxid

Thanks for helping. This project has an unusual shape, so read this before
opening a PR.

## What arxid is

arxid is not primarily a library. It is a **frozen specification**
([`spec/SPEC.md`](spec/SPEC.md)) plus **canonical test vectors**
([`vectors/vectors.json`](vectors/vectors.json)). The implementations exist to
serve the spec, not the other way round. The Rust crate is the reference
implementation; everything else is a port validated against the same vectors.

## The one rule

**The algorithm is frozen at spec v1.** Width, round count, the ARX constants
`7/13/17`, the golden constant, the key schedule, the base62 alphabet and its
order, and the code length are all part of the contract.

Any change to observable output requires a **new spec version and a new set of
vectors**. Never a silent bump. A PR that changes an existing vector value will
be rejected unless it is explicitly a spec-version proposal.

This includes behavior that looks like a bug but is specified. For example, the
key schedule maps `key` and `!key` to the same permutation (SPEC.md section
2.1). That is frozen. "Fixing" it makes your implementation non-conforming.

## Adding a port

This is the most valuable contribution you can make. The process:

**1. Read the spec and the hazards.** [`spec/SPEC.md`](spec/SPEC.md) is
normative. Section 8 lists the exact places ports silently diverge: wrapping
width, rotation width, logical vs arithmetic shift, and the 64-bit key schedule.

**2. Reimplement it natively.** Do not wrap the Rust crate, do not ship WASM,
do not transpile. A native port in idiomatic code for that language. Prefer zero
runtime dependencies; the core is pure integer arithmetic and needs nothing but
the standard library.

**3. Mirror the API shape.** Names follow the host language's conventions, but
the surface should be recognisable:

| Concept | Rust | TypeScript |
|---|---|---|
| Construct from key | `Arxid::new(key: u64)` | `new Arxid(key: bigint)` |
| Core | `obfuscate` / `deobfuscate` | `obfuscate` / `deobfuscate` |
| String layer | `obfuscate_str` / `deobfuscate_str` | `obfuscateStr` / `deobfuscateStr` |
| Decode failure | `Option<u64>` | `number \| null` |

The key is a 64-bit unsigned integer in the public API. Use the language's
native u64 type where one exists; where none does, use its arbitrary-precision
integer type. Do not take the key as bytes - that pushes an endianness decision
onto callers.

The core functions are **total**: out-of-domain integers are reduced with
`& MAX_ID`, never rejected. Only string decoding can fail, and it returns
"no value" rather than throwing.

**4. Validate against the vectors.** Load
[`vectors/vectors.json`](vectors/vectors.json) from the repo - do not copy it
into your port's directory, do not hand-transcribe values. For every row assert
all four directions:

```
obfuscate(id, key)           == obfuscated
deobfuscate(obfuscated, key) == id
to_base62(obfuscated)        == encoded
from_base62(encoded)         == obfuscated
```

Watch out for `key`: it ranges over the full `u64` and a float-based JSON parser
will round it. See [`vectors/README.md`](vectors/README.md).

**Round-trip tests are not sufficient.** They are symmetric and will pass on an
implementation that is wrong in a way that matters. The vectors are what proves
interoperability.

**5. Add a CI job.** Every port gets its own job in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml), and every job runs the
same vectors. The pattern is deliberately uniform - copy the `ts` job, change
the toolchain, keep the vector step. Adding a port should be additive: a new
job, no changes to existing ones.

**6. Open the PR.** Land it in `impl/<language>/`, update the ports table in
[`README.md`](README.md), and add a `CHANGELOG.md` entry.

**A port that does not pass the vectors is not merged.** There is no partial
credit here: a port that is 99% right is a port that silently corrupts ids
across service boundaries.

## Working on an existing implementation

Bug fixes, docs, tests, performance, and ergonomics are all welcome, as long as
observable output does not change. The vectors are the guard rail: if your
change makes them fail, the change is wrong.

### Rust (`impl/rust`)

```sh
cd impl/rust
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features --lib
```

The crate is `#![forbid(unsafe_code)]` and builds on `no_std`. If you add a
dependency, justify it: the core should need nothing but `core`, and the
encoding layer nothing but `alloc`.

### TypeScript (`impl/ts`)

```sh
cd impl/ts
npm install
npm run typecheck
npm test
npm run build
```

Zero runtime dependencies is a hard requirement.

## How changes land

`main` is protected. Every change arrives through a pull request, and the
required checks must pass before it can merge:

| Check | From |
|---|---|
| `all green` | the CI workflow (every implementation, every feature combination, the canonical vectors) |
| `cargo deny` | Rust dependency advisories, licenses, bans, sources |
| `codeql (typescript)` | static analysis |
| `npm audit` | JavaScript dependency advisories |

Merges are squash-only, and the branch is deleted afterwards. Force pushes and
deletion of `main` are blocked outright.

## Releasing

Maintainers only. Publishing is automated; do not run `cargo publish` or
`npm publish` by hand.

1. Bump the version in **both** `impl/rust/Cargo.toml` and
   `impl/ts/package.json`. They must match: the release workflow refuses to run
   if they disagree.
2. Update `CHANGELOG.md`.
3. Land those on `main` through a PR as usual.
4. Rehearse: run the **Release** workflow from the Actions tab with
   `dry_run: true`. This executes the entire path, including `cargo publish
   --dry-run` and `npm publish --dry-run`, without publishing.
5. Tag and push:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

The workflow then re-runs the full suite (a tag can point at a commit CI never
saw), verifies the tag matches both manifests, publishes to crates.io and npm,
and opens a GitHub release. npm packages are published with provenance, so the
tarball is cryptographically tied to the workflow run and commit that produced
it.

Publishing waits on the `release` environment, so a manual approval can be
required in Settings > Environments.

### Credentials

| Registry | How it authenticates |
|---|---|
| crates.io | `CARGO_REGISTRY_TOKEN` repository secret |
| npm | **no secret.** Trusted publishing (OIDC): npm trusts this repository, the `release.yml` workflow, and the `release` environment. The job's `id-token: write` permission mints a short-lived, workflow-scoped credential. |

Do not add `NODE_AUTH_TOKEN` to the npm job. Setting it overrides OIDC and
breaks trusted publishing.

Inspect or change the npm trust relationship with:

```sh
npx npm@latest trust list arxid
```

Note that `npm trust` needs a recent npm; the `--allow-publish` flag does not
exist in older versions, which fail with an opaque `400 Bad Request`.

## Changing the vectors

Only when the reference implementation changes, and only by regenerating:

```sh
cd impl/rust
cargo run --example gen_vectors > ../../vectors/vectors.json
```

Never edit `vectors.json` by hand. If regeneration changes an existing value,
stop: you have changed the algorithm, which is a spec-version decision, not a
patch.

## Proposing a spec v2

Open an issue first, not a PR. Include: what changes, why the current behavior
is inadequate, and the migration story for data already obfuscated under v1
(which is unrecoverable under a different algorithm). Spec versions are
expensive by design.

## Code of conduct

By participating you agree to the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Licensing

Contributions are accepted under the [MIT License](LICENSE). By submitting a
PR you confirm you have the right to license your contribution under those
terms.
