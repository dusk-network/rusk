import { get, writable } from "svelte/store";

/** @param {*} initialValue */
function mockReadableStore(initialValue) {
  const store = writable(initialValue);
  const { set, subscribe } = store;
  const getMockedStoreValue = () => get(store);

  /** @param {*} value */
  const setMockedStoreValue = (value) => set(value);

  return {
    getMockedStoreValue,
    setMockedStoreValue,
    subscribe,
  };
}

export default mockReadableStore;
