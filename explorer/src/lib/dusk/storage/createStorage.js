/**
 * @param {StorageType} type
 * @param {StorageSerializer} serializer
 * @param {StorageDeserializer} deserializer
 * @returns {DuskStorage}
 */
function createStorage(type, serializer, deserializer) {
  const storage = type === "local" ? localStorage : sessionStorage;

  return {
    async clear() {
      return storage.clear();
    },

    async getItem(key) {
      const value = storage.getItem(key);

      return value === null ? null : deserializer(value);
    },

    async removeItem(key) {
      return storage.removeItem(key);
    },

    async setItem(key, value) {
      return storage.setItem(key, serializer(value));
    },
  };
}

export default createStorage;
