import { createStorage } from "$lib/dusk/storage";

/**
 * @param {string} key
 * @param {any} value
 */
const reviver = (key, value) =>
  key === "lastUpdate" ? new Date(value) : value;
const storage = createStorage("local", JSON.stringify, (value) =>
  JSON.parse(value, reviver)
);
const key = "market-data";

export default {
  clear() {
    return storage.removeItem(key);
  },

  /** @returns {Promise<MarketData>} */
  get() {
    return storage.getItem(key);
  },

  /**
   *
   * @param {(evt: StorageEvent) => void} listener
   * @returns {(() => void)} The function to remove the listener.
   */
  onChange(listener) {
    /** @param {StorageEvent} evt */
    const handleStorageChange = (evt) => {
      if (evt.key === key) {
        listener(evt);
      }
    };

    window.addEventListener("storage", handleStorageChange);

    return () => window.removeEventListener("storage", handleStorageChange);
  },

  /** @param {MarketDataStorage} value */
  set(value) {
    return storage.setItem(key, value);
  },
};
