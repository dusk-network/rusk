/* eslint-disable no-extra-parens */

/** @type {(s: string) => Uint8Array} */
const base64ToBytes = s => Uint8Array.from(
	atob(s),
	/** @type {(c: string) => number} */ (c => c.codePointAt(0))
);

/* eslint-enable no-extra-parens */

export default base64ToBytes;
