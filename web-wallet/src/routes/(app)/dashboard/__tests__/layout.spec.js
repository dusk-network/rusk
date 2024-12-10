import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { act, cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { apiMarketData } from "$lib/mock-data";
import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";

import mockedWalletStore from "../../../../__mocks__/mockedWalletStore";

import Layout from "../+layout.svelte";
import { load } from "../+layout.js";

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {typeof import("$lib/stores")} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: { ...original.walletStore, ...mockedWalletStore },
  };
});

describe("Dashboard Layout", () => {
  /**
   * @param {Element} container
   * @param {"error" | "success" | "warning"} status
   * @returns
   */
  const getStatusWrapper = (container, status) =>
    container.querySelector(`.footer__network-status-icon--${status}`);
  const initialState = mockedWalletStore.getMockedStoreValue();

  beforeEach(() => {
    mockedWalletStore.setMockedStoreValue(initialState);
  });

  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/stores");
  });

  describe("Layout load", () => {
    const responseOK = new Response(JSON.stringify(apiMarketData), {
      status: 200,
    });
    const fetchMock = vi.fn().mockResolvedValue(responseOK);

    // @ts-ignore
    const loadData = () => load({ fetch: fetchMock });

    afterEach(() => {
      fetchMock.mockClear();
    });

    /* eslint-disable no-extra-parens */

    it('should return a promise that resolves with the data returned by the "quote" API', async () => {
      const result = /** @type {Record<string, any>} */ (await loadData());

      expect(fetchMock).toHaveBeenCalledTimes(1);
      expect(fetchMock).toHaveBeenCalledWith(
        "https://api.coingecko.com/api/v3/coins/dusk-network?community_data=false&developer_data=false&localization=false&market_data=true&sparkline=false&tickers=false"
      );

      expect(result.currentPrice).resolves.toStrictEqual(
        apiMarketData.market_data.current_price
      );
    });

    it("should return a promise that resolves with an empty object if the expected properties in the returned data are missing", async () => {
      /* eslint-disable camelcase */
      const wrongResponse1 = new Response(JSON.stringify({ market_data: {} }), {
        status: 200,
      });
      /* eslint-enable camelcase */
      const wrongResponse2 = new Response(JSON.stringify({}), { status: 200 });

      fetchMock
        .mockResolvedValueOnce(wrongResponse1)
        .mockResolvedValueOnce(wrongResponse2);

      const result1 = /** @type {Record<string, any>} */ (await loadData());
      const result2 = /** @type {Record<string, any>} */ (await loadData());

      expect(fetchMock).toHaveBeenCalledTimes(2);
      expect(fetchMock).toHaveBeenNthCalledWith(
        1,
        "https://api.coingecko.com/api/v3/coins/dusk-network?community_data=false&developer_data=false&localization=false&market_data=true&sparkline=false&tickers=false"
      );
      expect(fetchMock).toHaveBeenNthCalledWith(
        2,
        "https://api.coingecko.com/api/v3/coins/dusk-network?community_data=false&developer_data=false&localization=false&market_data=true&sparkline=false&tickers=false"
      );
      expect(result1.currentPrice).resolves.toStrictEqual({});
      expect(result2.currentPrice).resolves.toStrictEqual({});
    });

    it('should return a promise that resolves with an empty object if the fetch Response status is not "ok"', async () => {
      fetchMock.mockResolvedValueOnce(new Response("", { status: 404 }));

      const result = /** @type {Record<string, any>} */ (await loadData());

      expect(fetchMock).toHaveBeenCalledTimes(1);
      expect(fetchMock).toHaveBeenCalledWith(
        "https://api.coingecko.com/api/v3/coins/dusk-network?community_data=false&developer_data=false&localization=false&market_data=true&sparkline=false&tickers=false"
      );

      expect(result.currentPrice).resolves.toStrictEqual({});
    });

    it("should return a promise that resolves with an empty object if the fetch fails or the Response contains invalid JSON", async () => {
      fetchMock
        .mockRejectedValueOnce(new Error("some error"))
        .mockResolvedValueOnce(new Response("}", { status: 200 }));

      const result1 = /** @type {Record<string, any>} */ (await loadData());
      const result2 = /** @type {Record<string, any>} */ (await loadData());

      expect(fetchMock).toHaveBeenCalledTimes(2);
      expect(fetchMock).toHaveBeenNthCalledWith(
        1,
        "https://api.coingecko.com/api/v3/coins/dusk-network?community_data=false&developer_data=false&localization=false&market_data=true&sparkline=false&tickers=false"
      );
      expect(fetchMock).toHaveBeenNthCalledWith(
        2,
        "https://api.coingecko.com/api/v3/coins/dusk-network?community_data=false&developer_data=false&localization=false&market_data=true&sparkline=false&tickers=false"
      );
      expect(result1.currentPrice).resolves.toStrictEqual({});
      expect(result2.currentPrice).resolves.toStrictEqual({});
    });

    /* eslint-enable no-extra-parens */
  });

  const usdPrice = 0.5;
  const expectedFiat =
    (luxToDusk(get(mockedWalletStore).balance.shielded.value) +
      luxToDusk(get(mockedWalletStore).balance.unshielded.value)) *
    usdPrice;
  const formatter = createCurrencyFormatter("en", "usd", 2);
  const baseProps = {
    data: { currentPrice: Promise.resolve({ usd: usdPrice }) },
  };

  it("should render the dashboard layout", () => {
    const { container } = render(Layout, baseProps);

    expect(getStatusWrapper(container, "success")).toBeTruthy();
    expect(container).toMatchSnapshot();
  });

  it("should render the dashboard layout and show a throbber while balance is loading", async () => {
    vi.useFakeTimers();

    const { container } = render(Layout, baseProps);

    expect(container.querySelector(".dusk-balance__fiat--visible")).toBeNull();

    expect(container.firstChild).toMatchSnapshot();

    await vi.runAllTimersAsync();

    expect(
      container.querySelector(".dusk-balance__fiat--visible")
    ).toBeTruthy();

    expect(container.querySelector(".dusk-balance__fiat")).toHaveTextContent(
      formatter(expectedFiat)
    );

    vi.useRealTimers();
  });

  it("should render the dashboard layout in the sync state when no progress is reported", async () => {
    const { container } = render(Layout, baseProps);

    expect(getStatusWrapper(container, "warning")).toBeNull();

    await act(() => {
      mockedWalletStore.setMockedStoreValue({
        ...initialState,
        syncStatus: {
          error: null,
          from: 0n,
          isInProgress: true,
          last: 0n,
          progress: 0,
        },
      });
    });

    expect(getStatusWrapper(container, "warning")).toBeTruthy();
    expect(container.firstChild).toMatchSnapshot();

    await act(() => {
      mockedWalletStore.setMockedStoreValue(initialState);
    });

    expect(getStatusWrapper(container, "warning")).toBeNull();
  });

  it("should render the dashboard layout in the sync state with a progress indicator", async () => {
    const { container } = render(Layout, baseProps);

    expect(getStatusWrapper(container, "warning")).toBeNull();

    await act(() => {
      mockedWalletStore.setMockedStoreValue({
        ...initialState,
        syncStatus: {
          error: null,
          from: 100n,
          isInProgress: true,
          last: 200n,
          progress: 0.5,
        },
      });
    });

    expect(getStatusWrapper(container, "warning")).toBeTruthy();
    expect(container).toMatchSnapshot();

    await act(() => {
      mockedWalletStore.setMockedStoreValue(initialState);
    });

    expect(getStatusWrapper(container, "warning")).toBeNull();
  });

  it("should render the dashboard layout in the error state", async () => {
    const { container } = render(Layout, baseProps);
    const getRetryButton = () =>
      container.querySelector('[aria-label="Retry synchronization"]');

    expect(getStatusWrapper(container, "error")).toBeNull();
    expect(getRetryButton()).toBeNull();

    await act(() => {
      mockedWalletStore.setMockedStoreValue({
        ...initialState,
        syncStatus: {
          error: new Error(),
          from: 0n,
          isInProgress: false,
          last: 0n,
          progress: 0,
        },
      });
    });

    expect(getStatusWrapper(container, "error")).toBeTruthy();
    expect(getRetryButton()).toBeTruthy();
    expect(container).toMatchSnapshot();

    await act(() => {
      mockedWalletStore.setMockedStoreValue(initialState);
    });

    expect(getStatusWrapper(container, "error")).toBeNull();
    expect(getRetryButton()).toBeNull();
    expect(getStatusWrapper(container, "success")).toBeTruthy();
  });
});
