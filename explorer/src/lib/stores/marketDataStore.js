import { derived, get } from "svelte/store";
import { always, pickIn } from "lamb";

import { createPollingDataStore } from "$lib/dusk/svelte-stores";
import { duskAPI, marketDataStorage } from "$lib/services";

import appStore from "./appStore";

const fetchInterval = get(appStore).marketDataFetchInterval;
const pollingDataStore = createPollingDataStore(
  duskAPI.getMarketData,
  fetchInterval
);
const getStorage = () => marketDataStorage.get().catch(always(null));

/** @param {MarketDataStorage} value */
const setStorage = (value) =>
  marketDataStorage.set(value).catch(always(undefined));

/** @type {MarketDataStoreContent} */
const initialState = {
  ...get(pollingDataStore),
  lastUpdate: null,
  ...(await getStorage()),
};

const marketDataStore = derived(
  pollingDataStore,
  ($pollingDataStore, set) => {
    const current = get(marketDataStore);
    const isDataChanged = $pollingDataStore.data !== current.data;
    const hasNewData = $pollingDataStore.data && isDataChanged;
    const newStore = {
      data: $pollingDataStore.data ?? current.data,
      error: hasNewData ? null : $pollingDataStore.error ?? current.error,
      isLoading: $pollingDataStore.isLoading,
      lastUpdate: hasNewData
        ? new Date()
        : current.data
          ? current.lastUpdate
          : null,
    };

    if (hasNewData) {
      setStorage(
        /** @type {MarketDataStorage} */ (
          pickIn(newStore, ["data", "lastUpdate"])
        )
      );
    }

    set(newStore);
  },
  initialState
);

function isDataStale() {
  const { error, isLoading, lastUpdate } = get(marketDataStore);

  return (
    !!lastUpdate &&
    (error !== null || (!isLoading && Date.now() > +lastUpdate + fetchInterval))
  );
}

pollingDataStore.start();

/** @type {MarketDataStore} */
export default {
  isDataStale,
  subscribe: marketDataStore.subscribe,
};
