import { createDataStore } from "$lib/dusk/svelte-stores";
import { duskAPI } from "$lib/services";

const locationsDataStore = createDataStore(duskAPI.getNodeLocations);
locationsDataStore.getData();

/** @type {NodeLocationStore} */
export default {
  subscribe: locationsDataStore.subscribe,
};
