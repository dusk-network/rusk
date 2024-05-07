import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { act, cleanup } from "@testing-library/svelte";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";
import mockedWalletStore from "../../__mocks__/mockedWalletStore";
import Layout from "../+layout.svelte";
import { load } from "../+layout.js";

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {WalletStore} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: mockedWalletStore,
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
  const initialState = structuredClone(mockedWalletStore.getMockedStoreValue());

  beforeEach(() => {
    mockedWalletStore.setMockedStoreValue(initialState);
  });

  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/stores");
  });

  describe("Layout load", () => {
    const currentPrice = {
      /* eslint-disable camelcase */
      market_data: {
        current_price: {
          usd: 0.5,
        },
      },
      /* eslint-enable camelcase */
    };
    const responseOK = new Response(JSON.stringify(currentPrice), {
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
        "https://api.dusk.network/v1/quote"
      );

      expect(result.currentPrice).resolves.toStrictEqual(
        currentPrice.market_data.current_price
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
        "https://api.dusk.network/v1/quote"
      );
      expect(fetchMock).toHaveBeenNthCalledWith(
        2,
        "https://api.dusk.network/v1/quote"
      );
      expect(result1.currentPrice).resolves.toStrictEqual({});
      expect(result2.currentPrice).resolves.toStrictEqual({});
    });

    it('shoud return a promise that resolves with an empty object if the fetch Response status is not "ok"', async () => {
      fetchMock.mockResolvedValueOnce(new Response("", { status: 404 }));

      const result = /** @type {Record<string, any>} */ (await loadData());

      expect(fetchMock).toHaveBeenCalledTimes(1);
      expect(fetchMock).toHaveBeenCalledWith(
        "https://api.dusk.network/v1/quote"
      );

      expect(result.currentPrice).resolves.toStrictEqual({});
    });

    it("shoud return a promise that resolves with an empty object if the fetch fails or the Response contains invalid JSON", async () => {
      fetchMock
        .mockRejectedValueOnce(new Error("some error"))
        .mockResolvedValueOnce(new Response("}", { status: 200 }));

      const result1 = /** @type {Record<string, any>} */ (await loadData());
      const result2 = /** @type {Record<string, any>} */ (await loadData());

      expect(fetchMock).toHaveBeenCalledTimes(2);
      expect(fetchMock).toHaveBeenNthCalledWith(
        1,
        "https://api.dusk.network/v1/quote"
      );
      expect(fetchMock).toHaveBeenNthCalledWith(
        2,
        "https://api.dusk.network/v1/quote"
      );
      expect(result1.currentPrice).resolves.toStrictEqual({});
      expect(result2.currentPrice).resolves.toStrictEqual({});
    });

    /* eslint-enable no-extra-parens */
  });

  it("should render the dashboard layout", () => {
    const { container } = renderWithSimpleContent(Layout, {});

    expect(getStatusWrapper(container, "success")).toBeTruthy();
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the dashboard layout in the sync state", async () => {
    const { container } = renderWithSimpleContent(Layout, {});

    expect(getStatusWrapper(container, "warning")).toBeNull();

    await act(() => {
      mockedWalletStore.setMockedStoreValue({
        ...initialState,
        isSyncing: true,
      });
    });

    expect(getStatusWrapper(container, "warning")).toBeTruthy();
    expect(container.firstChild).toMatchSnapshot();

    await act(() => {
      mockedWalletStore.setMockedStoreValue({ initialState });
    });

    expect(getStatusWrapper(container, "warning")).toBeNull();
  });

  it("should render the dashboard layout in the error state", async () => {
    const { container } = renderWithSimpleContent(Layout, {});
    const getRetryButton = () =>
      container.querySelector(".footer__actions-button--retry");

    expect(getStatusWrapper(container, "error")).toBeNull();
    expect(getRetryButton()).toBeNull();

    await act(() => {
      mockedWalletStore.setMockedStoreValue({
        ...initialState,
        error: new Error(),
      });
    });

    expect(getStatusWrapper(container, "error")).toBeTruthy();
    expect(getRetryButton()).toBeTruthy();
    expect(container.firstChild).toMatchSnapshot();

    await act(() => {
      mockedWalletStore.setMockedStoreValue({ initialState });
    });

    expect(getStatusWrapper(container, "error")).toBeNull();
    expect(getRetryButton()).toBeNull();
    expect(getStatusWrapper(container, "success")).toBeTruthy();
  });
});
