/** @type {(s: string) => string} */
const ensureTrailingSlash = (s) => (s.endsWith("/") ? s : `${s}/`);

export default ensureTrailingSlash;
