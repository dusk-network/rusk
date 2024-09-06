import { settingsStore, walletStore } from "$lib/stores";
import { getSeedFromMnemonic } from "$lib/wallet";
import { getWallet } from "$lib/services/wallet";

/**
 * @param {string[]} mnemonicPhrase
 * @param {number | undefined} syncFrom
 */
async function initializeWallet(mnemonicPhrase, syncFrom = undefined) {
  settingsStore.reset();

  const mnemonic = mnemonicPhrase.join(" ");
  const seed = getSeedFromMnemonic(mnemonic);
  const wallet = getWallet(seed);
  walletStore.clearLocalDataAndInit(wallet, syncFrom);
}

export default initializeWallet;
