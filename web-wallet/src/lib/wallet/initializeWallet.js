import { settingsStore, walletStore } from "$lib/stores";

import getSeedFromMnemonic from "./getSeedFromMnemonic";
import profileGeneratorFrom from "./profileGeneratorFrom";

/**
 * @param {string} mnemonic
 * @param {bigint} [syncFrom]
 */
async function initializeWallet(mnemonic, syncFrom) {
  settingsStore.reset();

  const profileGenerator = await profileGeneratorFrom(
    getSeedFromMnemonic(mnemonic)
  );

  walletStore.clearLocalDataAndInit(profileGenerator, syncFrom);
}

export default initializeWallet;
