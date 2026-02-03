import { mnemonicToSeedSync } from "@scure/bip39";

/**
 * @param {String} mnemonic
 * @returns {Uint8Array<ArrayBuffer>}
 */
const getSeedFromMnemonic = (mnemonic) =>
  Uint8Array.from(mnemonicToSeedSync(mnemonic));

export default getSeedFromMnemonic;
