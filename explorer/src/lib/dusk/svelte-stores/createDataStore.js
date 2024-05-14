import { get, writable } from "svelte/store";

import { getErrorFrom } from "$lib/dusk/error";

/**
 * @param {(...args: any) => Promise<any>} dataRetriever
 * @returns {DataStore}
 */
function createDataStore(dataRetriever) {
  /** @type {DataStoreContent} */
  const initialState = {
    data: null,
    error: null,
    isLoading: false,
  };

  const dataStore = writable(initialState);
  const { set, subscribe, update } = dataStore;

  /** @type {number} */
  let currentRetrieveId = 0;

  /** @type {(...args: Parameters<dataRetriever>) => Promise<DataStoreContent>} */
  const getData = (...args) => {
    const retrieveId = ++currentRetrieveId;

    update((store) => ({ ...store, error: null, isLoading: true }));

    return dataRetriever(...args)
      .then((data) => {
        if (retrieveId === currentRetrieveId) {
          const newStoreContent = { data, error: null, isLoading: false };

          set(newStoreContent);

          return newStoreContent;
        } else {
          return get(dataStore);
        }
      })
      .catch(
        /** @param {any} error */
        (error) => {
          if (retrieveId === currentRetrieveId) {
            const newStoreContent = {
              data: null,
              error: getErrorFrom(error),
              isLoading: false,
            };

            set(newStoreContent);

            return newStoreContent;
          } else {
            return get(dataStore);
          }
        }
      );
  };

  const reset = () => {
    /**
     * We don't want pending promises to be written
     * in the store, and we don't want id clashes
     * if `getData` is called immediately after `reset`.
     */
    currentRetrieveId++;
    set(initialState);
  };

  return {
    getData,
    reset,
    subscribe,
  };
}

export default createDataStore;
