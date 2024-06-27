import { get, writable } from "svelte/store";

/** @type {NetworkOption[]}*/
const networks = [
  { label: "Testnet", value: import.meta.env.VITE_DUSK_TESTNET_NODE },
  { label: "Devnet", value: import.meta.env.VITE_DUSK_DEVNET_NODE },
];

/** @type {AppStoreContent} */
const initialState = {
  blocksListEntries: Number(import.meta.env.VITE_BLOCKS_LIST_ENTRIES),
  chainInfoEntries: Number(import.meta.env.VITE_CHAIN_INFO_ENTRIES),
  fetchInterval: Number(import.meta.env.VITE_REFETCH_INTERVAL),
  marketDataFetchInterval: Number(
    import.meta.env.VITE_MARKET_DATA_REFETCH_INTERVAL
  ),
  network: networks[0].value,
  networks,
  transactionsListEntries: Number(
    import.meta.env.VITE_TRANSACTIONS_LIST_ENTRIES
  ),
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
