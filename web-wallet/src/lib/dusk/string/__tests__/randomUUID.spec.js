import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { randomUUID as nodeRandomUUID } from "crypto";
import { range } from "lamb";

describe("randomUUID", () => {
  const originalCrypto = global.crypto;
  const uuidRE =
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

  afterEach(() => {
    vi.resetModules();
  });

  describe("with crypto API", () => {
    if (!originalCrypto) {
      Object.defineProperty(global, "crypto", {
        value: { randomUUID: nodeRandomUUID },
        writable: false,
      });

      afterAll(() => {
        global.crypto = originalCrypto;
      });
    }

    it("should generate a v4 random UUID using the native crypto API if present", async () => {
      const { randomUUID } = await import("..");
      const uuid = randomUUID();

      expect(uuid.length).toBe(36);
    });
  });

  describe("without crypto API", () => {
    it("should be able to generate a v4 random UUID without the crypto API", async () => {
      const cryptoSpy = originalCrypto
        ? vi
            .spyOn(global, "crypto", "get")
            //@ts-ignore
            .mockReturnValue(undefined)
        : null;

      const { randomUUID } = await import("..");
      const generated = new Set();

      range(0, 100, 1).forEach(() => {
        const uuid = randomUUID();

        expect(uuidRE.test(uuid)).toBe(true);

        generated.add(uuid);
      });

      expect(generated.size).toBe(100);

      cryptoSpy?.mockRestore();
    });

    it("should be able to generate a v4 random UUID if the crypto API is present, but its `randomUUID` function doesn't", async () => {
      let originalRandomUUID;

      if (originalCrypto) {
        originalRandomUUID = global.crypto.randomUUID;
        Object.defineProperty(global.crypto, "randomUUID", {
          value: undefined,
        });
      }

      const { randomUUID } = await import("..");
      const generated = new Set();

      range(0, 100, 1).forEach(() => {
        const uuid = randomUUID();

        expect(uuidRE.test(uuid)).toBe(true);

        generated.add(uuid);
      });

      expect(generated.size).toBe(100);

      if (originalRandomUUID) {
        Object.defineProperty(global.crypto, "randomUUID", {
          value: originalRandomUUID,
        });
      }
    });
  });
});
