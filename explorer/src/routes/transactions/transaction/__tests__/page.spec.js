import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformTransaction } from "$lib/chain-info";
import {
  apiMarketData,
  gqlTransaction,
  gqlTransactionDetails,
} from "$lib/mock-data";
import { appStore } from "$lib/stores";

import TransactionDetails from "../+page.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Transaction Details", () => {
  vi.useFakeTimers();

  const { fetchInterval } = get(appStore);
  const getTransactionSpy = vi
    .spyOn(duskAPI, "getTransaction")
    .mockResolvedValue(transformTransaction(gqlTransaction.tx));
  const getPayloadSpy = vi
    .spyOn(duskAPI, "getTransactionDetails")
    .mockResolvedValue(gqlTransactionDetails.tx.raw);
  const getMarketDataSpy = vi
    .spyOn(duskAPI, "getMarketData")
    .mockResolvedValue({
      currentPrice: apiMarketData.market_data.current_price,
      marketCap: apiMarketData.market_data.market_cap,
    });

  afterEach(() => {
    cleanup();
    getTransactionSpy.mockClear();
    getPayloadSpy.mockClear();
    getMarketDataSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getTransactionSpy.mockRestore();
    getPayloadSpy.mockRestore();
    getMarketDataSpy.mockRestore();
  });

  it("should render the Transaction details page, start polling the transaction data and stop the polling when unmounted", async () => {
    const { container, unmount } = render(TransactionDetails);

    expect(container.firstChild).toMatchSnapshot();

    expect(getTransactionSpy).toHaveBeenCalledTimes(1);
    expect(getPayloadSpy).toHaveBeenCalledTimes(1);
    expect(getMarketDataSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getTransactionSpy).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getTransactionSpy).toHaveBeenCalledTimes(3);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getTransactionSpy).toHaveBeenCalledTimes(3);
  });
});
