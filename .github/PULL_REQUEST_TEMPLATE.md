# What this changes

<!-- One or two sentences. Link the issue if there is one. -->

Closes #

## Type

- [ ] Bug fix (no change to observable output)
- [ ] New port (`impl/<language>`)
- [ ] Docs / tests / CI
- [ ] Spec version proposal (changes observable output - discuss in an issue first)

## Output compatibility

**The algorithm is fixed within a spec version (currently v2).** Confirm one:

- [ ] This does not change the output of any `(key, id)` pair.
- [ ] This is an intentional spec-version change, agreed in issue # above.

- [ ] `vectors/vectors.json` is unchanged, **or** it was regenerated with
      `cargo run --example gen_vectors` (never hand-edited) and the change is
      explained below.

## Checks

<!-- Delete the sections that do not apply. -->

Rust:

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-features` (includes the canonical vectors)

TypeScript:

- [ ] `npm run typecheck`
- [ ] `npm test` (includes the canonical vectors)
- [ ] `npm run build`

## For a new port

- [ ] Native reimplementation of `spec/SPEC.md` - not a WASM wrapper, not a
      transpile, not a binding
- [ ] Zero runtime dependencies (or the exceptions are justified below)
- [ ] Key exposed as a 64-bit unsigned integer (or the language's
      arbitrary-precision integer where none exists) - not as bytes
- [ ] Core functions are total: out-of-domain integers are reduced, never
      rejected; only string decoding can fail, and it returns "no value" rather
      than throwing
- [ ] Test suite loads `vectors/vectors.json` **from this repo** (not vendored,
      not transcribed) and asserts all four directions for every row
- [ ] `key` is parsed as a 64-bit integer, not through a float
- [ ] A CI job was added to `.github/workflows/ci.yml`, following the existing
      pattern, and added to the `all-green` job's `needs`
- [ ] The ports table in `README.md` was updated
- [ ] A `CHANGELOG.md` entry was added

## Notes

<!-- Anything reviewers should know: design decisions, trade-offs, follow-ups. -->
