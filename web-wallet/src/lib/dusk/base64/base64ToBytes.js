/** @type {(s: string) => Uint8Array} */
const base64ToBytes = (s) =>
  Uint8Array.from(
    atob(s),
    /** @type {(c: string) => number} */ ((c) => c.codePointAt(0))
  );

export default base64ToBytes;
