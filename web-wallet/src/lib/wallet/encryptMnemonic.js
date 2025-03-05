import encryptBuffer from "./encryptBuffer";

/**
 * @param {string} mnemonic
 * @param {string} pwd
 * @returns {Promise<WalletEncryptInfo>}
 */
const encryptMnemonic = async (mnemonic, pwd) =>
  await encryptBuffer(new TextEncoder().encode(mnemonic), pwd);

export default encryptMnemonic;
