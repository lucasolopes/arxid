/**
 * Structural tests: round-trip, totality, injectivity, and the JavaScript-
 * specific hazards from SPEC.md section 8. Conformance to the spec is proved by
 * `vectors.test.ts`, not here.
 */

import { describe, expect, it } from "vitest";

import {
  ALPHABET,
  Arxid,
  CODE_LEN,
  MAX_ID,
  ROUNDS,
  SPEC_VERSION,
  WIDTH_BITS,
  deobfuscate,
  fromBase62,
  obfuscate,
  toBase62,
} from "../src/index.js";

const SAMPLE_IDS = [
  0,
  1,
  2,
  42,
  1000,
  12345,
  1_000_000,
  Math.floor(MAX_ID / 2),
  MAX_ID - 1,
  MAX_ID,
];

// No pair here may be the bitwise complement of another: complement keys define
// the same permutation by design (SPEC.md section 2.1), so `u64::MAX` would
// duplicate key 0 and weaken the distinctness check below.
const SAMPLE_KEYS = [
  0n,
  1n,
  0x9e3779b97f4a7c15n,
  0xd1b54a32d192ed03n,
  1n << 63n, // only the top bit set: catches a key truncated to 32 bits
];

describe("constants", () => {
  it("match the spec", () => {
    expect(WIDTH_BITS).toBe(40);
    expect(ROUNDS).toBe(6);
    expect(MAX_ID).toBe(2 ** 40 - 1);
    expect(CODE_LEN).toBe(7);
    expect(SPEC_VERSION).toBe(2);
    expect(ALPHABET).toHaveLength(62);
    expect(ALPHABET.slice(0, 10)).toBe("0123456789");
    expect(ALPHABET[10]).toBe("A");
    expect(ALPHABET[35]).toBe("Z");
    expect(ALPHABET[36]).toBe("a");
    expect(ALPHABET[61]).toBe("z");
  });
});

describe("permutation", () => {
  it("round-trips across sampled ids and keys", () => {
    for (const key of SAMPLE_KEYS) {
      const arxid = new Arxid(key);
      for (const id of SAMPLE_IDS) {
        const code = arxid.obfuscate(id);
        expect(code).toBeLessThanOrEqual(MAX_ID);
        expect(code).toBeGreaterThanOrEqual(0);
        expect(Number.isSafeInteger(code)).toBe(true);
        expect(arxid.deobfuscate(code)).toBe(id);
      }
    }
  });

  it("round-trips a dense random sample", () => {
    const arxid = new Arxid(0xd1b54a32d192ed03n);
    for (let i = 0; i < 20_000; i += 1) {
      const id = Math.floor(Math.random() * (MAX_ID + 1));
      expect(arxid.deobfuscate(arxid.obfuscate(id))).toBe(id);
    }
  });

  it("is injective on a sampled range", () => {
    const arxid = new Arxid(0xd1b54a32d192ed03n);
    const seen = new Set<number>();
    for (let id = 0; id < 100_000; id += 1) {
      const code = arxid.obfuscate(id);
      expect(seen.has(code), `collision at id=${id}`).toBe(false);
      seen.add(code);
    }
  });

  it("uses the whole key, not just the low 32 bits", () => {
    // If a port truncated the key, these two would produce the same output.
    expect(obfuscate(12345, 0x1234_5678n)).not.toBe(
      obfuscate(12345, 0xffff_ffff_1234_5678n),
    );
  });

  it("uses the whole 40-bit domain, not just 32 bits", () => {
    const arxid = new Arxid(7n);
    const high = SAMPLE_IDS.filter((id) => id > 2 ** 32).map((id) =>
      arxid.obfuscate(id),
    );
    expect(high.length).toBeGreaterThan(0);
    expect(high.some((code) => code > 2 ** 32)).toBe(true);
  });

  it("gives distinct outputs for distinct keys", () => {
    for (const id of SAMPLE_IDS) {
      const outs = new Set(SAMPLE_KEYS.map((k) => obfuscate(id, k)));
      expect(outs.size, `keys collided on id=${id}`).toBe(SAMPLE_KEYS.length);
    }
  });

  it("no longer treats complement keys as equivalent", () => {
    // Spec v1 folded the key halves with XOR, which cancelled under complement
    // and collapsed the key space to 2^63. Spec v2 folds with a wrapping add.
    // Pinned so the regression cannot creep back in.
    const complement = (k: bigint) => ~k & ((1n << 64n) - 1n);
    for (const key of SAMPLE_KEYS) {
      const differs = SAMPLE_IDS.some(
        (id) => obfuscate(id, key) !== obfuscate(id, complement(key)),
      );
      expect(differs, `key ${key} still matches its complement`).toBe(true);
    }
  });

  it("does not preserve the ordering of consecutive ids", () => {
    // Deliberately not asserted: that adjacent codes never occur. See SPEC.md
    // section 11 - spec v1 made that claim and it was false.
    const arxid = new Arxid(0x0123456789abcdefn);
    const codes = Array.from({ length: 100 }, (_, i) => arxid.obfuscate(100 + i));
    let ascending = 0;
    for (let i = 1; i < codes.length; i += 1) {
      if (codes[i - 1]! < codes[i]!) ascending += 1;
    }
    expect(ascending).toBeGreaterThan(20);
    expect(ascending).toBeLessThan(80);
  });

  it("reduces out-of-domain input instead of rejecting it", () => {
    const key = 3n;
    expect(obfuscate(MAX_ID + 1, key)).toBe(obfuscate(0, key));
    expect(deobfuscate(obfuscate(MAX_ID + 1, key), key)).toBe(0);
    expect(obfuscate(-1, key)).toBe(obfuscate(MAX_ID, key));
    expect(obfuscate(Number.NaN, key)).toBe(obfuscate(0, key));
    expect(obfuscate(Number.POSITIVE_INFINITY, key)).toBe(obfuscate(0, key));
  });

  it("normalizes keys outside [0, 2^64)", () => {
    expect(obfuscate(42, -1n)).toBe(obfuscate(42, (1n << 64n) - 1n));
    expect(obfuscate(42, 1n << 64n)).toBe(obfuscate(42, 0n));
  });

  it("matches the class and free-function paths", () => {
    const key = 0x0123456789abcdefn;
    const arxid = new Arxid(key);
    for (const id of SAMPLE_IDS) {
      expect(arxid.obfuscate(id)).toBe(obfuscate(id, key));
      expect(arxid.deobfuscate(id)).toBe(deobfuscate(id, key));
    }
  });
});

describe("base62", () => {
  it("round-trips", () => {
    for (const n of [...SAMPLE_IDS, 61, 62, 3843, 62 ** 6]) {
      const s = toBase62(n);
      expect(s).toHaveLength(CODE_LEN);
      expect(fromBase62(s)).toBe(n);
    }
  });

  it("pads and orders digits as specified", () => {
    expect(toBase62(0)).toBe("0000000");
    expect(toBase62(1)).toBe("0000001");
    expect(toBase62(10)).toBe("000000A");
    expect(toBase62(35)).toBe("000000Z");
    expect(toBase62(36)).toBe("000000a");
    expect(toBase62(61)).toBe("000000z");
    expect(toBase62(62)).toBe("0000010");
  });

  it("rejects malformed input without throwing", () => {
    for (const s of ["", "abc", "aaaaaaaaaaaa", "!!!!!!!", "00000 0", "zzzzzzz"]) {
      expect(fromBase62(s)).toBeNull();
    }
  });

  it("rejects strings above MAX_ID", () => {
    expect(fromBase62(toBase62(MAX_ID))).toBe(MAX_ID);
    expect(fromBase62("zzzzzzz")).toBeNull();
  });

  it("returns null from deobfuscateStr on malformed input", () => {
    const arxid = new Arxid(1n);
    expect(arxid.deobfuscateStr("nope")).toBeNull();
    expect(arxid.deobfuscateStr("zzzzzzz")).toBeNull();
  });
});
