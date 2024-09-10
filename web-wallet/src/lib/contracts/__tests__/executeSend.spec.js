import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { walletStore } from "$lib/stores";
import { getLastTransactionHash } from "$lib/transactions";
import { executeSend } from "..";

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {typeof import("$lib/stores")} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: {
      ...original.walletStore,
      transfer: vi.fn().mockResolvedValue(undefined),
    },
  };
});

vi.mock("$lib/transactions", async (importOriginal) => {
  /** @type {typeof import("$lib/transactions")} */
  const original = await importOriginal();

  return {
    ...original,
    getLastTransactionHash: vi.fn(() => ""),
  };
});

afterEach(() => {
  vi.mocked(walletStore.transfer).mockClear();
  vi.mocked(getLastTransactionHash).mockClear();
});

afterAll(() => {
  vi.doUnmock("$lib/stores/walletStore");
  vi.doUnmock("$lib/transactions");
});

describe("executeSend", () => {
  it("should call the walletStore transfer method", async () => {
    const args = ["abc", 1000, 1, 2];
    // @ts-ignore
    await executeSend(...args);

    expect(walletStore.transfer).toHaveBeenCalledTimes(1);
    expect(walletStore.transfer).toHaveBeenCalledWith("abc", 1000, {
      limit: 2,
      price: 1,
    });
    expect(getLastTransactionHash).toHaveBeenCalledTimes(1);
  });

  it("should not call the getLastTransactionHash function when an error is emitted from the transfer function", async () => {
    const err = new Error("some error");

    vi.mocked(walletStore.transfer).mockRejectedValueOnce(err);

    await expect(executeSend("abc", 1000, 1, 2)).rejects.toBe(err);
    expect(getLastTransactionHash).not.toHaveBeenCalled();
  });
});
