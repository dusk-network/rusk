import { goto as svelteGoto } from "$app/navigation";

import addBasePath from "./addBasePath";

/** @type {(...args: Parameters<svelteGoto>) => ReturnType<svelteGoto>} */
const goto = (url, ...rest) => svelteGoto(addBasePath(url), ...rest);

export default goto;
