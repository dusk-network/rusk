import { writable } from "svelte/store";
import { AddressSyncer, Network } from "$lib/vendor/w3sper.js/src/mod";

/** @type {Network} */
const network = new Network(import.meta.env.VITE_WALLET_NETWORK);

/** @type {NetworkStoreContent} */
const initialState = {
  get connected() {
    return network.connected;
  },
};

const networkStore = writable(initialState);
const { subscribe } = networkStore;

/** @type {NetworkStoreServices["connect"]} */
const connect = async () => (network.connected ? network : network.connect());

/** @type {NetworkStoreServices["disconnect"]} */
const disconnect = () => network.disconnect();

/** @type {NetworkStoreServices["getCurrentBlockHeight"]} */
const getCurrentBlockHeight = () => network.blockHeight;

/** @type {(options?: NetworkSyncerOptions) => Promise<AddressSyncer>} */
const getAddressSyncer = (options) =>
  network.connect().then(() => new AddressSyncer(network, options));

/** @type {NetworkStore} */
export default {
  connect,
  disconnect,
  getAddressSyncer,
  getCurrentBlockHeight,
  subscribe,
};
