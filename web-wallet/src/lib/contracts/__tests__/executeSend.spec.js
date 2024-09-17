import { afterAll, afterEach, describe, expect, it, vi } from "vitest";

import { walletStore } from "$lib/stores";

import { executeSend } from "..";

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {typeof import("$lib/stores")} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: {
      ...original.walletStore,
      transfer: vi.fn().mockResolvedValue({
        hash: "some-tx-id",
        nullifiers: [],
      }),
    },
  };
});

afterEach(() => {
  vi.mocked(walletStore.transfer).mockClear();
});

afterAll(() => {
  vi.doUnmock("$lib/stores/walletStore");
});

describe("executeSend", () => {
  it("should call the walletStore transfer method and execute the transaction", async () => {
    const duskAmount = 1000;
    const luxAmount = BigInt(duskAmount * 1e9);
    const result = await executeSend("fake-address", duskAmount, 1, 500);

    expect(walletStore.transfer).toHaveBeenCalledTimes(1);
    expect(walletStore.transfer).toHaveBeenCalledWith(
      "fake-address",
      luxAmount,
      expect.objectContaining({
        limit: 500n,
        price: 1n,
      })
    );
    expect(result).toBe("some-tx-id");
  });

  it("should correctly convert decimal amounts in Dusk to Lux", async () => {
    const duskAmount = 1234.56789;
    const luxAmount = 1_234_567_890_000n;
    const result = await executeSend("fake-address", duskAmount, 1, 500);

    expect(walletStore.transfer).toHaveBeenCalledTimes(1);
    expect(walletStore.transfer).toHaveBeenCalledWith(
      "fake-address",
      luxAmount,
      expect.objectContaining({
        limit: 500n,
        price: 1n,
      })
    );
    expect(result).toBe("some-tx-id");
  });
});
