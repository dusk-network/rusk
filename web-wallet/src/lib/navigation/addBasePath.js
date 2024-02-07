import { base } from "$app/paths";

/**
 * @template {string | URL} T
 * @param {T} path
 */
const addBasePath = path => (
	path instanceof URL
		? path
		: path.startsWith("/") ? `${base}${path}` : path
);

export default addBasePath;
