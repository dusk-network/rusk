import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { duskAPI } from "$lib/services";
import { transformTransaction } from "$lib/chain-info";
import {
  apiMarketData,
  gqlTransaction,
  gqlTransactionDetails,
} from "$lib/mock-data";

import TransactionDetails from "../+page.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Transaction Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

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

  it("should render the Transaction details page and query the necessary info", async () => {
    const { container } = render(TransactionDetails);

    expect(container.firstChild).toMatchSnapshot();

    expect(getTransactionSpy).toHaveBeenCalledTimes(1);
    expect(getPayloadSpy).toHaveBeenCalledTimes(1);
    expect(getMarketDataSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();
  });
});
