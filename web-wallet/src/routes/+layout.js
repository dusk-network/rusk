import "$lib/dusk/polyfill/asyncIterator";
import "$lib/dusk/polyfill/promiseWithResolvers";

// eslint-disable-next-line import/no-unresolved
import "web-streams-polyfill/polyfill";

export const csr = true;
export const prerender = true;
export const ssr = false;
export const trailingSlash = "always";
