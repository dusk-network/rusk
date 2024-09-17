import "$lib/dusk/polyfill/asyncIterator";

import { networkStore } from "$lib/stores";

export const csr = true;
export const prerender = true;
export const ssr = false;
export const trailingSlash = "always";

/** @type {import('./$types').LayoutLoad} */
export async function load() {
  await networkStore.connect();
}
