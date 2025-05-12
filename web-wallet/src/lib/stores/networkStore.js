import { writable } from "svelte/store";
import { browser } from "$app/environment";
import { always, condition, getKey, getPath, isUndefined, when } from "lamb";
import {
  AccountSyncer,
  AddressSyncer,
  Network,
  useAsProtocolDriver,
} from "@dusk/w3sper";

import { rejectAfter } from "$lib/dusk/promise";
import { makeNodeUrl } from "$lib/url";

import wasmPath from "$lib/vendor/dusk_wallet_core.wasm?url";

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

/**
 * Checks if a block with the given height and hash
 * exists on the network.
 *
 * @type {NetworkStoreServices["checkBlock"]}
 */
const checkBlock = (height, hash) =>
  connect()
    .then(() => network.query(`checkBlock(height: ${height}, hash: "${hash}")`))
    .then(getKey("checkBlock"));

/** @type {NetworkStoreServices["connect"]} */
const connect = async () =>
  network.connected
    ? network
    : Promise.race([
        fetch(wasmPath)
          .then((response) => response.arrayBuffer())
          .then((buffer) => {
            useAsProtocolDriver(new Uint8Array(buffer));

            return network.connect();
          }),
        rejectAfter(
          10000,
          new Error("Timed out while connecting to the network")
        ),
      ]);

/** @type {NetworkStoreServices["disconnect"]} */
const disconnect = () => network.disconnect();

/** @type {() => Promise<AccountSyncer>} */
const getAccountSyncer = () => connect().then(() => new AccountSyncer(network));

/** @type {() => Promise<AddressSyncer>} */
const getAddressSyncer = () => connect().then(() => new AddressSyncer(network));

/** @type {NetworkStoreServices["getBlockHashByHeight"]} */
const getBlockHashByHeight = (height) =>
  connect()
    .then(() => network.query(`block(height: ${height}) { header { hash } }`))
    .then(getPath("block.header.hash"))
    .then(when(isUndefined, always("")));

/** @type {NetworkStoreServices["getCurrentBlockHeight"]} */
const getCurrentBlockHeight = () => network.blockHeight;

/** @type {NetworkStoreServices["getLastFinalizedBlockHeight"]} */
const getLastFinalizedBlockHeight = () =>
  connect()
    .then(() => network.query("lastBlockPair { json }"))
    .then(getPath("lastBlockPair.json.last_finalized_block.0"))
    .then(condition(isUndefined, always(0n), BigInt));

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
  checkBlock,
  connect,
  disconnect,
  getAccountSyncer,
  getAddressSyncer,
  getBlockHashByHeight,
  getCurrentBlockHeight,
  getLastFinalizedBlockHeight,
  init,
  subscribe,
};
