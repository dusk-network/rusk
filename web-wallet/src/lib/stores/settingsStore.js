import { persisted } from "svelte-persisted-store";
import { browser } from "$app/environment";

const initialState = {
	currency: "USD",
	darkMode: false,
	dashboardTransactionLimit: 5,
	gasLimit: parseInt(import.meta.env.VITE_GAS_LIMIT_DEFAULT, 10),
	gasLimitLower: parseInt(import.meta.env.VITE_GAS_LIMIT_LOWER, 10),
	gasLimitUpper: parseInt(import.meta.env.VITE_GAS_LIMIT_UPPER, 10),
	gasPrice: parseInt(import.meta.env.VITE_GAS_PRICE_DEFAULT, 10),
	gasPriceLower: parseInt(import.meta.env.VITE_GAS_PRICE_LOWER, 10),
	hideStakingNotice: false,
	language: browser ? navigator.language : "en",
	network: "testnet",
	userId: ""
};
const settingsStore = persisted(`${CONFIG.LOCAL_STORAGE_APP_KEY}-preferences`, initialState);
const { set, subscribe, update } = settingsStore;

function reset () {
	set(initialState);
}

export default {
	reset,
	set,
	subscribe,
	update
};
