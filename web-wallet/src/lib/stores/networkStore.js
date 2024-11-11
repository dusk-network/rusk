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
      new URL("https://localhost")
    );
  }
}

const networkUrl = getNetworkUrl();

/** @type {Network} */
const network = new Network(networkUrl);

/** @type {NetworkStoreContent} */
const initialState = {
  get connected() {
    return network.connected;
  },
  networkName: "unknown",
};

const networkStore = writable(initialState);
const { set, subscribe } = networkStore;

/** @type {NetworkStoreServices["connect"]} */
const connect = async () => (network.connected ? network : network.connect());

/** @type {NetworkStoreServices["disconnect"]} */
const disconnect = () => network.disconnect();

/** @type {NetworkStoreServices["getCurrentBlockHeight"]} */
const getCurrentBlockHeight = () => network.blockHeight;

/** @type {() => Promise<AccountSyncer>} */
const getAccountSyncer = () => connect().then(() => new AccountSyncer(network));

/** @type {(options?: NetworkSyncerOptions) => Promise<AddressSyncer>} */
const getAddressSyncer = (options) =>
  connect().then(() => new AddressSyncer(network, options));

/** @type {NetworkStoreServices["init"]} */
async function init() {
  const info = await network.node.info;

  set({
    ...initialState,
    networkName: info.chain.toString(),
  });
}

/** @type {NetworkStore} */
export default {
  connect,
  disconnect,
  getAccountSyncer,
  getAddressSyncer,
  getCurrentBlockHeight,
  init,
  subscribe,
};
