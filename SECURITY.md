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

| Version | Supported |
|---|---|
| 0.1.x | ✅ |

While the project is pre-1.0, only the latest release receives fixes.

## What arxid is and is not

Read this before filing a report - it defines what counts as a vulnerability.

arxid is a **keyed reversible permutation for id obfuscation**. It defeats
trivial enumeration of sequential resources. It is:

- **not** encryption of arbitrary data,
- **not** a MAC. It provides no authentication and no integrity. A valid-looking
  code does not prove it was produced by a holder of the key. Any well-formed
  7-character code decodes to *some* id. If you need unforgeability, layer a
  real MAC on top,
- **not** access control. ID obfuscation is not authorization. Never use arxid
  as the sole barrier protecting a resource that must stay secret,
- **not independently audited.** This construction has had no external
  cryptographic review. Non-enumerability is a measured statistical property
  (avalanche/SAC over a reduced-round ARX permutation), not a cryptographic
  guarantee. Resistance to key recovery depends on the round count and
  construction, and has not been formally analysed.

Deployments should use their own random key, kept out of source control.

### Known and accepted properties

These are documented, specified behavior. Reports about them are welcome as
discussion but are not treated as vulnerabilities:

- **`key` and `!key` produce the same permutation** (SPEC.md section 2.1), so
  the effective key space is 2^63 rather than 2^64. This is frozen in spec v1.
- **4 rounds** is a calibrated minimum for the avalanche target, not a
  cryptographic security margin.
- **The 40-bit domain** is small by cryptographic standards. It is a
  format-preserving choice driven by the 7-character code length.

### What is in scope

- A conforming-looking implementation that produces output disagreeing with
  `vectors/vectors.json` (an interop break can corrupt data across services).
- A practical key-recovery attack materially better than brute force.
- A panic, crash, unbounded allocation, or non-termination on any input, in any
  implementation. The core functions are specified as total.
- Key material leaking through logs, `Debug`/`toString` output, or error
  messages.
