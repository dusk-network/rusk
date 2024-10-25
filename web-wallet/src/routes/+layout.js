import "$lib/dusk/polyfill/asyncIterator";

// eslint-disable-next-line import/no-unresolved
import "web-streams-polyfill/polyfill";

import { networkStore } from "$lib/stores";

export const csr = true;
export const prerender = true;
export const ssr = false;
export const trailingSlash = "always";

/** @type {import('./$types').LayoutLoad} */
export async function load() {
  await networkStore.connect();
}
