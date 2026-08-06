# Security Policy

## Reporting a vulnerability

**Do not open a public issue for a security problem.**

Report privately through GitHub's
[private vulnerability reporting](https://github.com/lucasolopes/arxid/security/advisories/new)
(Security tab → Report a vulnerability), or by email to
**olopes.lucas567@gmail.com**.

Please include:

- what the issue is and why it matters,
- the affected implementation and version (`impl/rust`, `impl/ts`, ...),
- a reproduction: input key and id, expected vs actual output, or a short
  script.

You will get an acknowledgement within 7 days and an assessment within 30. This
is a small, unfunded project; there is no bounty program. Fixes ship as a
patch release with a GitHub Security Advisory.

## Supported versions

| Version | Spec | Supported |
|---|---|---|
| 0.2.x | v2 | ✅ |
| 0.1.x | v1 | ❌ withdrawn |

While the project is pre-1.0, only the latest release receives fixes.

**0.1.x is withdrawn, not merely superseded.** Its 4-round construction is
separable from a random permutation at about 2^13 chosen queries, and it mapped
consecutive ids to adjacent codes hundreds to thousands of times more often than
chance. Both are fixed in spec v2. Codes issued under v1 do not decode under v2; migrating means
re-issuing them.

## What arxid is and is not

Read this before filing a report - it defines what counts as a vulnerability.
The full threat model is [SPEC.md section 9](spec/SPEC.md).

arxid is a **keyed reversible permutation for id obfuscation**. It defeats
walking `/orders/1`, `/orders/2`, and it hides row counts and growth rates. It
does **not** make ids secret: recovering an id from a code without the key is
not computationally infeasible. It is:

- **not** encryption of arbitrary data,
- **not** a MAC. It provides no authentication and no integrity. A valid-looking
  code does not prove it was produced by a holder of the key. Any well-formed
  7-character code decodes to *some* id. If you need unforgeability, layer a
  real MAC on top,
- **not** access control. ID obfuscation is not authorization. Never use arxid
  as the sole barrier protecting a resource that must stay secret,
- **not independently audited.** This construction has had no external
  cryptographic review. The round function has no published lineage and no
  differential or linear analysis. Non-enumerability is a measured statistical
  property, not a cryptographic guarantee.

Deployments should use their own random key, kept out of source control, and a
separate key per resource type (there is no tweak).

### Known and accepted properties

These are documented, specified behavior. Reports about them are welcome as
discussion but are not treated as vulnerabilities:

- **The round count is a measured threshold plus the literature's minimum, not a
  security proof.** Three separate measurements stop showing structure by 5
  rounds: worst-pair avalanche deviation (0.1117 at 4 rounds, 0.0039 at 5, 0.0041
  at 6), a chosen-query distinguisher, and the adjacency rate. arxid ships 6, the
  minimum Patarin's generic attacks on Feistel schemes recommend for a
  pseudorandom permutation. Meeting that minimum is not a proof for this
  construction, whose round function is not a PRF. Reproducible with
  `examples/avalanche.rs`, `examples/distinguisher.rs`, and
  `examples/adjacency.rs`.
- **The 40-bit domain** is small by cryptographic standards, and the 20-bit
  Feistel halves are smaller. Luby-Rackoff only guarantees security to the
  birthday bound in the half width, about 2^10 queries, and does not apply here
  regardless because the ARX round function is not a PRF. Both are
  format-preserving choices driven by the 7-character code length.
- **No tweak.** One key defines one global mapping, so the same id yields the
  same code across resource types sharing that key. Use a separate key per type
  (SPEC.md section 2.2).
- **Consecutive ids can land on adjacent codes.** This is expected of any good
  permutation, at a rate of about two pairs per full domain. Spec v1 claimed
  otherwise; that claim has been withdrawn as false.

### What is in scope

- A conforming-looking implementation that produces output disagreeing with
  `vectors/vectors.json` (an interop break can corrupt data across services).
- A practical key-recovery attack materially better than brute force.
- A panic, crash, unbounded allocation, or non-termination on any input, in any
  implementation. The core functions are specified as total.
- Key material leaking through logs, `Debug`/`toString` output, or error
  messages.
