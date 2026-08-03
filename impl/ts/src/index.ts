/**
 * arxid - keyed, reversible permutation for obfuscating sequential integer IDs.
 *
 * Native TypeScript port of `spec/SPEC.md` (spec v1). This is not a WASM
 * wrapper: it reimplements the algorithm and is validated against the canonical
 * vectors in `/vectors/vectors.json`.
 *
 * The 40-bit public domain fits in `number` (well under 2^53), so ids and codes
 * are plain numbers. The key schedule operates on 64 bits and therefore uses
 * `BigInt`; the key is a `bigint`.
 */

/** Width of the permutation domain, in bits. */
export const WIDTH_BITS = 40;

/** Number of Feistel rounds. Frozen by spec v1. */
export const ROUNDS = 4;

/** Largest value in the domain: `2^40 - 1`. */
export const MAX_ID = 1099511627775;

/** Length of an encoded code, in characters. */
export const CODE_LEN = 7;

/** The specification version this implementation conforms to. */
export const SPEC_VERSION = 1;

/**
 * The base62 alphabet, in normative order: `0-9`, then `A-Z`, then `a-z`.
 * This order is part of the contract; do not reorder it.
 */
export const ALPHABET =
  "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

const TWO_POW_40 = 1099511627776;
const MASK_64 = (1n << 64n) - 1n;
const GOLDEN = 0x9e3779b97f4a7c15n;

/**
 * Reduces any number into `[0, MAX_ID]`, mirroring the reference's `& MAX_ID`.
 *
 * Bitwise `&` is unusable here: JavaScript's bitwise operators coerce to signed
 * int32, which would silently discard the top 8 bits of the domain.
 */
function reduce40(x: number): number {
  if (!Number.isFinite(x)) return 0;
  const t = Math.trunc(x) % TWO_POW_40;
  return t < 0 ? t + TWO_POW_40 : t;
}

/** Circular left rotation of a 64-bit value. */
function rotl64(v: bigint, n: bigint): bigint {
  const s = n & 63n;
  const masked = v & MASK_64;
  if (s === 0n) return masked;
  return ((masked << s) | (masked >> (64n - s))) & MASK_64;
}

/** Circular left rotation of a 32-bit value, returned unsigned. */
function rotl32(v: number, n: number): number {
  return ((v << n) | (v >>> (32 - n))) >>> 0;
}

/**
 * Derives the 32-bit round subkey from the 64-bit master key (SPEC.md section 4).
 *
 * This is the step that breaks naive ports: the whole computation is 64-bit, so
 * it runs in `BigInt` and only narrows to a 32-bit `number` at the very end.
 */
function subkey(key: bigint, round: number): number {
  const rotated = rotl64(key, BigInt((round * 7 + 1) % 64));
  const golden = (GOLDEN * BigInt(round + 1)) & MASK_64;
  const x = (rotated ^ golden) & MASK_64;
  return Number((x ^ (x >> 32n)) & 0xffffffffn) >>> 0;
}

/**
 * Derives every round subkey for a key. Exposed so both the class and the
 * free functions share one code path.
 */
function deriveSubkeys(key: bigint, rounds: number): number[] {
  const normalized = key & MASK_64;
  const out: number[] = [];
  for (let round = 0; round < rounds; round += 1) {
    out.push(subkey(normalized, round));
  }
  return out;
}

/**
 * ARX round function (SPEC.md section 5). Every add wraps modulo 2^32 and every
 * rotation is over 32 bits; the mask down to the half width happens only at the
 * end.
 */
function roundFn(r: number, rk: number, halfMask: number): number {
  let x = (r + rk) >>> 0;
  x = (x ^ rotl32(x, 7)) >>> 0;
  x = (x + rotl32(x, 13)) >>> 0;
  x = (x ^ rotl32(x, 17)) >>> 0;
  return (x & halfMask) >>> 0;
}

/**
 * Feistel forward pass over `width` bits.
 *
 * Split and recombine use arithmetic, not shifts: `>>` would truncate to 32
 * bits and destroy the top of a 40-bit value.
 */
function feistelN(
  input: number,
  subkeys: readonly number[],
  width: number,
): number {
  const half = width / 2;
  const halfPow = 2 ** half;
  const halfMask = halfPow - 1;
  let l = Math.floor(input / halfPow) % halfPow;
  let r = input % halfPow;
  for (let round = 0; round < subkeys.length; round += 1) {
    const f = roundFn(r, subkeys[round]!, halfMask);
    const newL = r;
    const newR = (l ^ f) >>> 0;
    l = newL;
    r = newR;
  }
  return l * halfPow + r;
}

/** Feistel inverse pass: the exact inverse of {@link feistelN}. */
function feistelNInv(
  input: number,
  subkeys: readonly number[],
  width: number,
): number {
  const half = width / 2;
  const halfPow = 2 ** half;
  const halfMask = halfPow - 1;
  let l = Math.floor(input / halfPow) % halfPow;
  let r = input % halfPow;
  for (let round = subkeys.length - 1; round >= 0; round -= 1) {
    const rPrev = l;
    const f = roundFn(rPrev, subkeys[round]!, halfMask);
    const lPrev = (r ^ f) >>> 0;
    l = lPrev;
    r = rPrev;
  }
  return l * halfPow + r;
}

/** Maps one alphabet character code to its digit value, or -1 if invalid. */
function digitValue(code: number): number {
  if (code >= 0x30 && code <= 0x39) return code - 0x30; // '0'-'9'
  if (code >= 0x41 && code <= 0x5a) return code - 0x41 + 10; // 'A'-'Z'
  if (code >= 0x61 && code <= 0x7a) return code - 0x61 + 36; // 'a'-'z'
  return -1;
}

/**
 * Encodes a 40-bit integer as a 7-character base62 string, most significant
 * digit first, left-padded with `'0'`.
 *
 * Values above {@link MAX_ID} are silently truncated to 7 digits; callers must
 * reduce to the domain first (everything in this module's public API does).
 *
 * @example
 * toBase62(0);  // "0000000"
 * toBase62(62); // "0000010"
 */
export function toBase62(n: number): string {
  let v = Number.isFinite(n) ? Math.trunc(n) : 0;
  if (v < 0) v = 0;
  const buf: string[] = ["0", "0", "0", "0", "0", "0", "0"];
  let i = CODE_LEN;
  while (v > 0 && i > 0) {
    i -= 1;
    buf[i] = ALPHABET[v % 62]!;
    v = Math.floor(v / 62);
  }
  return buf.join("");
}

/**
 * Decodes a 7-character base62 string back to its integer value.
 *
 * Returns `null` (never throws) when the string is not exactly 7 characters,
 * contains a character outside the alphabet, or decodes above {@link MAX_ID}.
 *
 * @example
 * fromBase62("0000000"); // 0
 * fromBase62("zzzzzzz"); // null (above MAX_ID)
 */
export function fromBase62(s: string): number | null {
  if (typeof s !== "string" || s.length !== CODE_LEN) return null;
  let n = 0;
  for (let i = 0; i < CODE_LEN; i += 1) {
    const d = digitValue(s.charCodeAt(i));
    if (d < 0) return null;
    n = n * 62 + d;
  }
  return n <= MAX_ID ? n : null;
}

/**
 * Obfuscates `id` under `key`. Values outside `[0, MAX_ID]` are reduced modulo
 * 2^40; never throws.
 *
 * Prefer {@link Arxid} when using one key repeatedly: it derives the round
 * subkeys once.
 */
export function obfuscate(id: number, key: bigint): number {
  return feistelN(reduce40(id), deriveSubkeys(key, ROUNDS), WIDTH_BITS);
}

/**
 * Recovers the original id from an obfuscated `code` under `key`. Values
 * outside `[0, MAX_ID]` are reduced modulo 2^40; never throws.
 */
export function deobfuscate(code: number, key: bigint): number {
  return feistelNInv(reduce40(code), deriveSubkeys(key, ROUNDS), WIDTH_BITS);
}

/**
 * A keyed, reversible permutation over the 40-bit id domain.
 *
 * @example
 * const arxid = new Arxid(0x9e3779b97f4a7c15n);
 * const code = arxid.obfuscate(1001);
 * arxid.deobfuscate(code); // 1001
 * arxid.obfuscateStr(1001); // 7-character base62 string
 */
export class Arxid {
  readonly #subkeys: readonly number[];

  /**
   * Builds a permutation from a 64-bit unsigned key.
   *
   * The key is a `bigint` because a `u64` does not fit safely in `number`.
   * Values outside `[0, 2^64)` are reduced modulo 2^64. Every bit of the key
   * participates in the schedule. Use a random key per deployment and keep it
   * out of source control.
   */
  constructor(key: bigint) {
    this.#subkeys = deriveSubkeys(key, ROUNDS);
  }

  /**
   * Obfuscates an id. Values outside `[0, MAX_ID]` are reduced modulo 2^40;
   * never throws.
   */
  obfuscate(id: number): number {
    return feistelN(reduce40(id), this.#subkeys, WIDTH_BITS);
  }

  /**
   * Recovers the original id from an obfuscated code. Values outside
   * `[0, MAX_ID]` are reduced modulo 2^40; never throws.
   */
  deobfuscate(code: number): number {
    return feistelNInv(reduce40(code), this.#subkeys, WIDTH_BITS);
  }

  /** Obfuscates an id and encodes it as a 7-character base62 string. */
  obfuscateStr(id: number): string {
    return toBase62(this.obfuscate(id));
  }

  /**
   * Decodes a 7-character base62 string and recovers the original id, or
   * `null` when the string is not a valid code.
   *
   * Any well-formed code decodes to *some* id: this is a permutation, not a
   * MAC. It does not tell you whether the code was produced with this key.
   */
  deobfuscateStr(s: string): number | null {
    const code = fromBase62(s);
    return code === null ? null : this.deobfuscate(code);
  }
}
