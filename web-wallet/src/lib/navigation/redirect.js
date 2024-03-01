import { redirect as svelteKitRedirect } from "@sveltejs/kit";

import addBasePath from "./addBasePath";

/**
 * @param {Parameters<svelteKitRedirect>[0]} status
 * @param {Parameters<svelteKitRedirect>[1]} path
 * @returns {ReturnType<svelteKitRedirect>}
 */
const redirect = (status, path) => svelteKitRedirect(status, addBasePath(path));

export default redirect;
