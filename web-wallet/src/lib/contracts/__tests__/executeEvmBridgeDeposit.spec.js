import { afterAll, afterEach, describe, expect, it, vi } from "vitest";

import { walletStore } from "$lib/stores";

import { executeEvmBridgeDeposit } from "..";

const VITE_EVM_BRIDGE_ID = import.meta.env.VITE_EVM_BRIDGE_ID;

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {typeof import("$lib/stores")} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: {
      ...original.walletStore,
      contractCall: vi.fn().mockResolvedValue({
        hash: "some-tx-id",
        nullifiers: [],
      }),
    },
  };
});

afterEach(() => {
  vi.mocked(walletStore.contractFunctionCall).mockClear();
});

afterAll(() => {
  vi.doUnmock("$lib/stores/walletStore");
});

describe("executeDeposit", () => {
  it("should call the walletStore contractCall method and execute the transaction", async () => {
    const duskAmount = 1000;
    const luxAmount = BigInt(duskAmount * 1e9);
    const result = await executeEvmBridgeDeposit(luxAmount, 1n, 500n);

    expect(walletStore.contractFunctionCall).toHaveBeenCalledTimes(1);
    expect(walletStore.contractFunctionCall).toHaveBeenCalledWith(
      luxAmount,
      expect.objectContaining({
        limit: 500n,
        price: 1n,
      }),
      VITE_EVM_BRIDGE_ID,
      "/src/lib/vendor/standard_bridge_dd_opt.wasm"
    );
    expect(result).toBe("some-tx-id");
  });
});
