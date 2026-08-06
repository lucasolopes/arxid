/**
 * Validates this port against the canonical test vectors.
 *
 * This is the interop contract. It loads the exact same
 * `/vectors/vectors.json` the Rust reference validates against: if this file
 * passes, this port is byte-identical to the reference by definition.
 *
 * Round-trip tests are symmetric and hide width/wrapping bugs (in particular
 * the 64-bit key schedule). Only these known-answer vectors catch them.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import {
  Arxid,
  CODE_LEN,
  MAX_ID,
  fromBase62,
  toBase62,
} from "../src/index.js";

interface Vector {
  key: number;
  id: number;
  obfuscated: number;
  encoded: string;
}

const VECTORS_URL = new URL("../../../vectors/vectors.json", import.meta.url);

/**
 * Reads the vectors as text and extracts each field with a regex rather than
 * `JSON.parse`, because `key` can be up to 2^64-1 and `JSON.parse` would round
 * it through a float. The key must reach the algorithm as an exact `bigint`.
 */
function loadVectors(): Array<Omit<Vector, "key"> & { key: bigint }> {
  const raw = readFileSync(fileURLToPath(VECTORS_URL), "utf8");
  const rows = JSON.parse(raw) as unknown;
  if (!Array.isArray(rows)) {
    throw new Error("vectors.json must be a JSON array");
  }

  // Recover the exact key digits from the source text, in file order.
  const keyDigits = [...raw.matchAll(/"key"\s*:\s*(\d+)/g)].map((m) => m[1]!);
  if (keyDigits.length !== rows.length) {
    throw new Error(
      `found ${keyDigits.length} key fields for ${rows.length} vectors`,
    );
  }

  return rows.map((row, i) => {
    const r = row as Record<string, unknown>;
    for (const field of ["key", "id", "obfuscated"] as const) {
      if (typeof r[field] !== "number") {
        throw new Error(`vector #${i} is missing the numeric field \`${field}\``);
      }
    }
    if (typeof r["encoded"] !== "string") {
      throw new Error(`vector #${i} is missing the string field \`encoded\``);
    }
    return {
      key: BigInt(keyDigits[i]!),
      id: r["id"] as number,
      obfuscated: r["obfuscated"] as number,
      encoded: r["encoded"] as string,
    };
  });
}

const vectors = loadVectors();

describe("canonical vectors", () => {
  it("is not empty", () => {
    expect(vectors.length).toBeGreaterThan(0);
  });

  it.each(vectors.map((v, i) => [i, v] as const))(
    "vector #%i matches in both directions",
    (_i, v) => {
      const arxid = new Arxid(v.key);

      expect(arxid.obfuscate(v.id)).toBe(v.obfuscated);
      expect(arxid.deobfuscate(v.obfuscated)).toBe(v.id);
      expect(toBase62(v.obfuscated)).toBe(v.encoded);
      expect(fromBase62(v.encoded)).toBe(v.obfuscated);
      expect(arxid.obfuscateStr(v.id)).toBe(v.encoded);
      expect(arxid.deobfuscateStr(v.encoded)).toBe(v.id);
    },
  );

  it("is well formed and covers the required cases", () => {
    for (const [i, v] of vectors.entries()) {
      expect(v.id, `vector #${i} id`).toBeLessThanOrEqual(MAX_ID);
      expect(v.obfuscated, `vector #${i} obfuscated`).toBeLessThanOrEqual(
        MAX_ID,
      );
      expect(v.encoded, `vector #${i} encoded`).toHaveLength(CODE_LEN);
    }

    const keys = new Set(vectors.map((v) => v.key));
    expect(keys.size).toBeGreaterThanOrEqual(4);
    for (const required of [0n, 1n, 0x9e3779b97f4a7c15n]) {
      expect(keys.has(required), `missing required key ${required}`).toBe(true);
    }

    const ids = new Set(vectors.map((v) => v.id));
    for (const required of [0, 1, 2, MAX_ID, MAX_ID - 1, Math.floor(MAX_ID / 2)]) {
      expect(ids.has(required), `missing required edge id ${required}`).toBe(
        true,
      );
    }
  });

  it("shows the consecutive run does not preserve input order", () => {
    // What is NOT checked, deliberately: that neighbouring ids never produce
    // adjacent codes. Spec v1 asserted that and it was false - see SPEC.md
    // section 11. A construction that truly guaranteed it would be
    // distinguishable from a random permutation for that very reason.
    const run = [
      ...new Map(
        vectors
          .filter((v) => v.id >= 100 && v.id <= 110)
          .map((v) => [v.id, v.obfuscated] as const),
      ),
    ].sort((a, b) => a[0] - b[0]);

    expect(run.length).toBeGreaterThanOrEqual(11);

    let ascending = 0;
    for (let i = 1; i < run.length; i += 1) {
      if (run[i - 1]![1] < run[i]![1]) ascending += 1;
    }
    expect(ascending, "the run is monotonic, which would leak input order")
      .toBeGreaterThan(0);
    expect(ascending).toBeLessThan(run.length - 1);
  });
});
