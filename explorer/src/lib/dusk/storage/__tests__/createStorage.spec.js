import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createStorage } from "..";

describe("createStorage", () => {
  const serializer = vi.fn(JSON.stringify);
  const deserializer = vi.fn(JSON.parse);

  afterEach(() => {
    serializer.mockClear();
    deserializer.mockClear();
  });

  for (const type of /** @type {StorageType[]} */ (["local", "session"])) {
    const storage = createStorage(type, serializer, deserializer);
    const systemStorage = globalThis[`${type}Storage`];
    const valueA = { a: 1, b: 2 };
    const serializedA = JSON.stringify(valueA);
    const valueB = ["a", "b", "c", "d"];
    const serializedB = JSON.stringify(valueB);

    beforeEach(() => {
      systemStorage.setItem("some-key", serializedA);
    });

    it("should expose a method to clear the created storage", async () => {
      await expect(storage.clear()).resolves.toBe(undefined);

      expect(systemStorage.length).toBe(0);
    });

    it("should expose a method to retrieve a value from the created storage", async () => {
      await expect(storage.getItem("some-key")).resolves.toStrictEqual(valueA);

      expect(deserializer).toHaveBeenCalledTimes(1);
      expect(deserializer).toHaveBeenCalledWith(serializedA);

      await expect(storage.getItem("some-other-key")).resolves.toBe(null);

      expect(deserializer).toHaveBeenCalledTimes(1);
    });

    it("should expose a method to remove a value from the created storage", async () => {
      await expect(storage.removeItem("some-key")).resolves.toBe(undefined);

      expect(systemStorage.getItem("some-key")).toBe(null);
    });

    it("should expose a method to set a value in the selected storage", async () => {
      await expect(storage.setItem("some-key", valueB)).resolves.toBe(
        undefined
      );

      expect(serializer).toHaveBeenCalledTimes(1);
      expect(serializer).toHaveBeenCalledWith(valueB);
      expect(systemStorage.getItem("some-key")).toBe(serializedB);
    });

    it("should return a rejected promise if any of the underlying storage method fails", async () => {
      /** @type {Array<keyof DuskStorage & keyof Storage>} */
      const methods = ["clear", "getItem", "removeItem", "setItem"];
      const error = new Error("some error message");

      for (const method of methods) {
        const methodSpy = vi
          .spyOn(Storage.prototype, method)
          .mockImplementation(() => {
            throw error;
          });

        // we don't care for correct parameters here
        await expect(
          storage[method]("some-key", "some value")
        ).rejects.toStrictEqual(error);

        methodSpy.mockRestore();
      }
    });
  }
});
