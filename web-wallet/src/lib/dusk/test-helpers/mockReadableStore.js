import { get, writable } from "svelte/store";

/**
 * @template T
 * @param {T} initialValue
 */
function mockReadableStore(initialValue) {
  const store = writable(initialValue);
  const { set, subscribe } = store;
  const getMockedStoreValue = () => get(store);

  /** @param {T} value */
  const setMockedStoreValue = (value) => set(value);

  return {
    getMockedStoreValue,
    setMockedStoreValue,
    subscribe,
  };
}

export default mockReadableStore;
