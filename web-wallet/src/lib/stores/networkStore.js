import { writable } from "svelte/store";
import { browser } from "$app/environment";
import {
  AccountSyncer,
  AddressSyncer,
  Network,
} from "$lib/vendor/w3sper.js/src/mod";
import { makeNodeUrl } from "$lib/url";

function getNetworkUrl() {
  if (browser) {
    return makeNodeUrl();
  } else {
    return (
      (import.meta.env.VITE_NODE_URL &&
        new URL(import.meta.env.VITE_NODE_URL)) ||
      "https://localhost"
    );
  }
}

/** @type {Network} */
const network = new Network(getNetworkUrl());

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

/** @type {(options?: NetworkSyncerOptions) => Promise<AccountSyncer>} */
const getAccountSyncer = (options) =>
  network.connect().then(() => new AccountSyncer(network, options));

/** @type {(options?: NetworkSyncerOptions) => Promise<AddressSyncer>} */
const getAddressSyncer = (options) =>
  network.connect().then(() => new AddressSyncer(network, options));

/** @type {NetworkStore} */
export default {
  connect,
  disconnect,
  getAccountSyncer,
  getAddressSyncer,
  getCurrentBlockHeight,
  subscribe,
};
