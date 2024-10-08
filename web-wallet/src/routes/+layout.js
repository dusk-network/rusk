import * as ProtocolDriver from "$lib/../../../w3sper.js/src/protocol-driver/mod";

import "$lib/dusk/polyfill/asyncIterator";

import { networkStore } from "$lib/stores";

export const csr = true;
export const prerender = true;
export const ssr = false;
export const trailingSlash = "always";

/** @type {import('./$types').LayoutLoad} */
export async function load({ fetch }) {
  // TODO Loading a local WASM for now
  const wasmUrl = new URL(
    "$lib/../../../target/wasm32-unknown-unknown/release/wallet_core.wasm",
    import.meta.url
  );

  await fetch(wasmUrl)
    .then((r) => r.arrayBuffer())
    .then((buffer) => ProtocolDriver.load(new Uint8Array(buffer)))
    .then(() => networkStore.connect());
}
