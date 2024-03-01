import { mnemonicToSeedSync } from "bip39";

/**
 * @param {String} mnemonic
 * @returns {Uint8Array}
 */
const getSeedFromMnemonic = (mnemonic) =>
  Uint8Array.from(mnemonicToSeedSync(mnemonic));

export default getSeedFromMnemonic;
