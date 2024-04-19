import { get, writable } from "svelte/store";

/** @type {NetworkOption[]}*/
const networks = [
  { label: "Testnet", value: import.meta.env.VITE_DUSK_TESTNET_NODE },
  { label: "Devnet", value: import.meta.env.VITE_DUSK_DEVNET_NODE },
];

/** @type {AppStoreContent} */
const initialState = {
  fetchInterval: Number(import.meta.env.VITE_REFETCH_INTERVAL),
  network: networks[0].value,
  networks,
};

const store = writable(initialState);
const { set, subscribe } = store;

/** @param {string} network */
const setNetwork = (network) =>
  set({
    ...get(store),
    network,
  });

/** @type {AppStore} */
export default {
  setNetwork,
  subscribe,
};
