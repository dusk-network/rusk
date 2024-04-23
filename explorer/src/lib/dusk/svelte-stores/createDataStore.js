import { writable } from "svelte/store";

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

  /** @type {ReturnType<dataRetriever> | null} */
  let dataPromise = null;

  /** @type {(...args: Parameters<dataRetriever>) => Promise<DataStoreContent>} */
  const getData = (...args) => {
    if (dataPromise) {
      return dataPromise;
    }

    update((store) => ({ ...store, error: null, isLoading: true }));

    dataPromise = dataRetriever(...args)
      .then((data) => {
        const newStore = { data, error: null, isLoading: false };

        set(newStore);

        return newStore;
      })
      .catch(
        /** @param {any} error */
        (error) => {
          const newStore = {
            data: null,
            error: getErrorFrom(error),
            isLoading: false,
          };

          set(newStore);

          return newStore;
        }
      )
      .finally(() => {
        dataPromise = null;
      });

    return dataPromise;
  };

  return {
    getData,
    subscribe,
  };
}

export default createDataStore;
