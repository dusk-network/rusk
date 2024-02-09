import { settingsStore, walletStore } from "$lib/stores";
import { getSeedFromMnemonic } from "$lib/wallet";
import { getWallet } from "$lib/services/wallet";
import { setKey } from "lamb";

/** @param {string[]} mnemonicPhrase */
async function initializeWallet (mnemonicPhrase) {
	settingsStore.reset();

	const mnemonic = mnemonicPhrase.join(" ");
	const seed = getSeedFromMnemonic(mnemonic);
	const wallet = getWallet(seed);
	const defaultAddress = (await wallet.getPsks())[0];

	await walletStore.clearLocalDataAndInit(wallet);

	settingsStore.update(setKey("userId", defaultAddress));
}

export default initializeWallet;
