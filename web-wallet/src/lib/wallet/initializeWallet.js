import { settingsStore, walletStore } from "$lib/stores";

import getSeedFromMnemonic from "./getSeedFromMnemonic";
import profileGeneratorFrom from "./profileGeneratorFrom";

/**
 * @param {string} mnemonic
 * @param {bigint | undefined} syncFrom
 */
async function initializeWallet(mnemonic, syncFrom = undefined) {
  settingsStore.reset();

  const profileGenerator = profileGeneratorFrom(getSeedFromMnemonic(mnemonic));

  walletStore.clearLocalDataAndInit(profileGenerator, syncFrom);
}

export default initializeWallet;
