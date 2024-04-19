import { onDestroy } from "svelte";

import { appStore } from "$lib/stores";

/**
 * @param {(network: string) => any} fn
 */
function onNetworkChange(fn) {
  /** @type {string} */
  let network;

  const unsubscribe = appStore.subscribe(($appStore) => {
    if ($appStore.network !== network) {
      network = $appStore.network;

      fn(network);
    }
  });

  onDestroy(unsubscribe);
}

export default onNetworkChange;
