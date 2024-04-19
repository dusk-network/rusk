import { derived, get } from "svelte/store";
import { isNull, keySatisfies, when } from "lamb";

import { resolveAfter } from "$lib/dusk/promise";

import { createDataStore } from ".";

/**
 * @param {(...args: any) => Promise<any>} dataRetriever
 * @param {number} fetchInterval
 * @returns {PollingDataStore}
 */
const createPollingDataStore = (dataRetriever, fetchInterval) => {
  /** @type {boolean} */
  let isPolling = false;

  const dataStore = createDataStore(dataRetriever);

  /** @type {(...args: any) => void} */
  const poll = (...args) => {
    if (isPolling) {
      dataStore
        .getData(...args)
        .then(
          when(keySatisfies(isNull, "error"), () =>
            resolveAfter(fetchInterval, undefined).then(() => poll(...args))
          )
        )
        .catch(stop);
    }
  };

  const stop = () => {
    isPolling = false;
  };

  /** @type {(...args: any) => void} */
  const start = (...args) => {
    if (isPolling) {
      return;
    }

    isPolling = true;
    poll(...args);
  };

  const pollingDataStore = derived(
    dataStore,
    ($dataStore, set) => {
      set($dataStore);
    },
    get(dataStore)
  );

  return {
    start,
    stop,
    subscribe: pollingDataStore.subscribe,
  };
};

export default createPollingDataStore;
