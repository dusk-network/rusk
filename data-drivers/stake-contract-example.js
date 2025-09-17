/* eslint-disable no-console */
/* eslint-disable camelcase */
/* eslint-disable no-var */

// Stake Contract Data-Driver Examples & Explainers
//
// This script demonstrates how to use the data-driver WASM from JavaScript.
// It highlights the most common tasks you'll do when talking to a Dusk contract:
//   1) Load the compiled data-driver WASM
//   2) Inspect driver metadata (schema + version)
//   3) Encode JSON inputs -> RKYV bytes for a contract call
//   4) Decode RKYV bytes -> JSON (for inputs, outputs, and events)
//   5) Handle errors defensively
//
// You can run it with: `npm start` (see package.json).
// Notes:
// - This script does **not** invoke the on-chain contract. It only (de)serializes
//   data according to the stake contract's ABI via the data-driver.
// - The paths assume you've built the WASM via `make data-drivers-js`.
// - Any base64 printed to the console is just a convenient way to view raw bytes.
import { loadDriverWasm } from "./data-driver/loader.js";
import { readFile } from "fs/promises";

function decodeBase64(b64) {
  const binary = Buffer.from(b64, "base64");
  return new Uint8Array(binary.buffer, binary.byteOffset, binary.byteLength);
}

function encodeBase64(bytes) {
  return Buffer.from(bytes).toString("base64");
}

/* eslint-disable max-statements */
async function run() {
  const wasmBytes = await readFile(
    "../../target/wasm32-unknown-unknown/release/dusk_stake_contract_dd_opt.wasm"
  );
  var driver = await loadDriverWasm(wasmBytes);
  driver.init();

  const stake_b64 =
    "AAAAAAAAAACoQy0KhXF6r0cZvvT6zvwFpZag9GMAMeazJO0MX/r2r90F80ajS0unX23QOyVFyxeQCSB42tRIptVk8IFawqTlcn5zDLoX/7+WXDsUjr44qxHcSaf9S1DwVG6+468r9BbJGQae8FT6WJNRqAhvfkUYH69u+9vGDuXtWasyng9ox7W8FDtxQJbkS6PlAHOJKQ7GDU9FZ2+micS803PKAdJYcxTCDEnLNP9h4p0UHj+a0Q+xH2MG0dEWejQBqRsO0wEAAAAAAAAAAKhDLQqFcXqvRxm+9PrO/AWllqD0YwAx5rMk7Qxf+vav3QXzRqNLS6dfbdA7JUXLF5AJIHja1Eim1WTwgVrCpOVyfnMMuhf/v5ZcOxSOvjirEdxJp/1LUPBUbr7jryv0FskZBp7wVPpYk1GoCG9+RRgfr27728YO5e1ZqzKeD2jHtbwUO3FAluRLo+UAc4kpDsYNT0Vnb6aJxLzTc8oB0lhzFMIMScs0/2HinRQeP5rRD7EfYwbR0RZ6NAGpGw7TAQAAAAAAAAAAVwotmqYDAABlBIKl6kZEXOPD81kdn2ljSl5ysk9gVVGasdz7Qb5jKBQw9pSY21oFDx0/i+5D8wVJjIiBu4G8PXmWdl6d5NGxtdYQc/j8pMamboRD3vGt5E/zvgpOqlIifs4axZlr9gcAAAAAAAAAAGUEgqXqRkRc48PzWR2faWNKXnKyT2BVUZqx3PtBvmMoFDD2lJjbWgUPHT+L7kPzBUmMiIG7gbw9eZZ2Xp3k0bG11hBz+PykxqZuhEPe8a3kT/O+Ck6qUiJ+zhrFmWv2BwAAAAAAAAAAAQAAAAAAAAA=";
  console.log("origStakeRkyv:", stake_b64);
  console.log();
  try {
    // --- Driver metadata ---
    // Introspect the contract interface from JS for quick validation/UX.
    const schema = driver.getSchema();
    const version = driver.getVersion();
    console.log("getSchema():", JSON.stringify(schema, null, 2));
    console.log("getVersion():", version);
    console.log();

    const stakeJson = driver.decodeInputFn("stake", decodeBase64(stake_b64));
    console.log("decodeInputFn:", stakeJson);
    console.log();

    const stakeEncoded = driver.encodeInputFn(
      "stake",
      JSON.stringify(stakeJson)
    );
    console.log("encodeInputFn:", encodeBase64(stakeEncoded));
  } catch (e) {
    console.error("FFI error:", e.message);
  }
}

run();
