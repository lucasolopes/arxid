// Verifies a batch of randomized rows produced by the Rust reference
// implementation (`cargo run --example gen_fuzz`) against the built TypeScript
// port. Any disagreement is an interop break: the two implementations would
// hand each other codes that decode to different ids.
//
//   node scripts/differential.mjs rows.json
//
// `key` is read from the raw JSON text, not through JSON.parse, because it
// spans the full u64 range and JSON.parse would silently round it to a double.

import { readFileSync } from "node:fs";
import { Arxid, toBase62, fromBase62 } from "../dist/index.js";

const path = process.argv[2];
if (!path) {
  console.error("usage: node scripts/differential.mjs <rows.json>");
  process.exit(2);
}

const raw = readFileSync(path, "utf8");

const seed = raw.match(/"seed":\s*(\d+)/)?.[1] ?? "?";
const rowRe =
  /\{\s*"key":\s*(\d+),\s*"id":\s*(\d+),\s*"obfuscated":\s*(\d+),\s*"encoded":\s*"([^"]+)"\s*\}/g;

let checked = 0;
const failures = [];

for (const m of raw.matchAll(rowRe)) {
  const key = BigInt(m[1]);
  const id = Number(m[2]);
  const expectedObfuscated = Number(m[3]);
  const expectedEncoded = m[4];

  const arxid = new Arxid(key);
  const got = arxid.obfuscate(id);

  if (got !== expectedObfuscated) {
    failures.push(
      `obfuscate(${id}) under key ${key}: rust ${expectedObfuscated}, ts ${got}`,
    );
  } else if (arxid.deobfuscate(got) !== id) {
    failures.push(`deobfuscate did not round-trip id ${id} under key ${key}`);
  } else if (toBase62(got) !== expectedEncoded) {
    failures.push(
      `toBase62(${got}): rust "${expectedEncoded}", ts "${toBase62(got)}"`,
    );
  } else if (fromBase62(expectedEncoded) !== expectedObfuscated) {
    failures.push(`fromBase62("${expectedEncoded}") disagreed`);
  } else if (arxid.deobfuscateStr(expectedEncoded) !== id) {
    failures.push(`deobfuscateStr("${expectedEncoded}") under key ${key}`);
  }

  checked += 1;
  if (failures.length >= 10) break;
}

if (checked === 0) {
  console.error(`no rows parsed from ${path} - the generator format changed?`);
  process.exit(2);
}

if (failures.length > 0) {
  console.error(
    `differential check FAILED: ${failures.length}+ mismatches in ${checked} rows (seed ${seed})`,
  );
  for (const f of failures) console.error(`  ${f}`);
  console.error(
    `\nreproduce: cargo run --release --example gen_fuzz -- ${seed} > rows.json`,
  );
  process.exit(1);
}

console.log(`differential check ok: ${checked} rows agree (seed ${seed})`);
