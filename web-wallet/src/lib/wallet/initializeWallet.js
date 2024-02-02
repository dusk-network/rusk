import { settingsStore, walletStore } from "$lib/stores";
import { getSeedFromMnemonic } from "$lib/wallet";
import { getWallet } from "$lib/services/wallet";

/** @param {string[]} mnemonicPhrase */
async function initializeWallet (mnemonicPhrase) {
	settingsStore.reset();

	const mnemonic = mnemonicPhrase.join(" ");
	const seed = getSeedFromMnemonic(mnemonic);
	const wallet = getWallet(seed);

	await walletStore.clearLocalDataAndInit(wallet);
}

export default initializeWallet;
