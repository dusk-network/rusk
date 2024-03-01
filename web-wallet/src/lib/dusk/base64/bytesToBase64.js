/** @type {(bytes: Uint8Array) => string} */
const bytesToBase64 = (bytes) => btoa(String.fromCodePoint(...bytes));

export default bytesToBase64;
