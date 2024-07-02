import { derived, get } from "svelte/store";

import { createPollingDataStore } from "$lib/dusk/svelte-stores";
import { duskAPI } from "$lib/services";

import { appStore } from ".";

const fetchInterval = get(appStore).marketDataFetchInterval;
const pollingDataStore = createPollingDataStore(
  duskAPI.getMarketData,
  fetchInterval
);

/** @type {MarketDataStoreContent} */
const initialState = {
  ...get(pollingDataStore),
  lastUpdate: null,
};

const marketDataStore = derived(
  pollingDataStore,
  ($pollingDataStore, set) => {
    const current = get(marketDataStore);
    const isDataChanged = $pollingDataStore.data !== current.data;
    const isErrorChanged = $pollingDataStore.error !== current.error;
    const hasNewData = $pollingDataStore.data && isDataChanged;

    set({
      data: $pollingDataStore.data ?? current.data,
      error: hasNewData ? null : $pollingDataStore.error ?? current.error,
      isLoading: $pollingDataStore.isLoading,
      lastUpdate: hasNewData
        ? new Date()
        : current.data
          ? current.lastUpdate
          : null,
    });

    if ($pollingDataStore.error && isErrorChanged) {
      pollingDataStore.start();
    }
  },
  initialState
);

pollingDataStore.start();

/** @type {MarketDataStore} */
export default {
  subscribe: marketDataStore.subscribe,
};
