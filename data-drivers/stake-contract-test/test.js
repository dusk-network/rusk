import { loadDriverWasm } from '../data-driver/loader.mjs';
import { readFile } from 'fs/promises';

function decodeBase64(b64) {
  const binary = Buffer.from(b64, 'base64');
  return new Uint8Array(binary.buffer, binary.byteOffset, binary.byteLength);
}

function encodeBase64(bytes) {
  return Buffer.from(bytes).toString("base64");
}

async function run() {
  const wasmBytes = await readFile('../../target/wasm32-unknown-unknown/release/dusk_stake_contract_dd_opt.wasm');
  var driver = await loadDriverWasm(wasmBytes);
  driver.init();

  const input = { amount: 100 };

  const stake_b64 = "AAAAAAAAAACoQy0KhXF6r0cZvvT6zvwFpZag9GMAMeazJO0MX/r2r90F80ajS0unX23QOyVFyxeQCSB42tRIptVk8IFawqTlcn5zDLoX/7+WXDsUjr44qxHcSaf9S1DwVG6+468r9BbJGQae8FT6WJNRqAhvfkUYH69u+9vGDuXtWasyng9ox7W8FDtxQJbkS6PlAHOJKQ7GDU9FZ2+micS803PKAdJYcxTCDEnLNP9h4p0UHj+a0Q+xH2MG0dEWejQBqRsO0wEAAAAAAAAAAKhDLQqFcXqvRxm+9PrO/AWllqD0YwAx5rMk7Qxf+vav3QXzRqNLS6dfbdA7JUXLF5AJIHja1Eim1WTwgVrCpOVyfnMMuhf/v5ZcOxSOvjirEdxJp/1LUPBUbr7jryv0FskZBp7wVPpYk1GoCG9+RRgfr27728YO5e1ZqzKeD2jHtbwUO3FAluRLo+UAc4kpDsYNT0Vnb6aJxLzTc8oB0lhzFMIMScs0/2HinRQeP5rRD7EfYwbR0RZ6NAGpGw7TAQAAAAAAAAAAVwotmqYDAABlBIKl6kZEXOPD81kdn2ljSl5ysk9gVVGasdz7Qb5jKBQw9pSY21oFDx0/i+5D8wVJjIiBu4G8PXmWdl6d5NGxtdYQc/j8pMamboRD3vGt5E/zvgpOqlIifs4axZlr9gcAAAAAAAAAAGUEgqXqRkRc48PzWR2faWNKXnKyT2BVUZqx3PtBvmMoFDD2lJjbWgUPHT+L7kPzBUmMiIG7gbw9eZZ2Xp3k0bG11hBz+PykxqZuhEPe8a3kT/O+Ck6qUiJ+zhrFmWv2BwAAAAAAAAAAAQAAAAAAAAA=";
  console.log("origStakeRkyv:", stake_b64);
  console.log();
  try {
    console.log("Datadriver Version:", driver.getVersion());
    console.log();

    const stakeJson = driver.decodeInputFn("stake", decodeBase64(stake_b64));
    console.log("decodeInputFn:", stakeJson);
    console.log();

    const stakeEncoded = driver.encodeInputFn("stake", JSON.stringify(stakeJson));
    console.log("encodeInputFn:", encodeBase64(stakeEncoded));



  } catch (e) {
    console.error("FFI error:", e.message);
  }
}

run();