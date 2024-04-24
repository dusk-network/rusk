import { afterAll, afterEach, describe, expect, it, vi } from "vitest";

import * as mockData from "$lib/mock-data";

import {
  transformBlock,
  transformSearchResult,
  transformTransaction,
} from "$lib/chain-info";

import { duskAPI } from "..";

describe("duskAPI", () => {
  const fetchSpy = vi.spyOn(global, "fetch");
  const node = "nodes.dusk.network";
  const fakeID = "some-id";
  const apiGetOptions = {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
    },
    method: "GET",
  };

  const endpointEnvName = "VITE_API_ENDPOINT";

  /** @type {(endpoint: string) => URL} */
  const getExpectedURL = (endpoint) =>
    new URL(`${import.meta.env[endpointEnvName]}/${endpoint}?node=${node}`);

  /** @type {(data: Record<string | number, any>) => Response} */
  const makeOKResponse = (data) =>
    new Response(JSON.stringify(data), { status: 200 });

  afterEach(() => {
    fetchSpy.mockClear();
  });

  afterAll(() => {
    fetchSpy.mockRestore();
  });

  it("should expose a method to retrieve a single block", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiBlock));

    expect(duskAPI.getBlock(node, fakeID)).resolves.toStrictEqual(
      transformBlock(mockData.apiBlock.data.blocks[0])
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL(`blocks/${fakeID}`),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the list of blocks", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiBlocks));

    expect(duskAPI.getBlocks(node)).resolves.toStrictEqual(
      mockData.apiBlocks.data.blocks.map(transformBlock)
    );

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("blocks"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the latest chain info", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiLatestChainInfo));

    expect(duskAPI.getLatestChainInfo(node)).resolves.toStrictEqual({
      blocks: mockData.apiLatestChainInfo.data.blocks.map(transformBlock),
      transactions:
        mockData.apiLatestChainInfo.data.transactions.map(transformTransaction),
    });
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("latest"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the market data", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiMarketData));

    expect(duskAPI.getMarketData()).resolves.toStrictEqual({
      currentPrice: mockData.apiMarketData.market_data.current_price,
      marketCap: mockData.apiMarketData.market_data.market_cap,
    });
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      new URL(
        getExpectedURL("quote")
          .toString()
          .replace(/(\?).+$/, "$1")
      ),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the node locations", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiNodeLocations));

    expect(duskAPI.getNodeLocations(node)).resolves.toStrictEqual(
      mockData.apiNodeLocations.data
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("locations"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the statistics", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiStats));

    expect(duskAPI.getStats(node)).resolves.toStrictEqual(mockData.apiStats);
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("stats"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve a single transaction", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiTransaction));

    expect(duskAPI.getTransaction(node, fakeID)).resolves.toStrictEqual(
      transformTransaction(mockData.apiTransaction.data[0])
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL(`transactions/${fakeID}`),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the details of a single transaction", () => {
    fetchSpy.mockResolvedValueOnce(
      makeOKResponse(mockData.apiTransactionDetails)
    );

    expect(duskAPI.getTransactionDetails(node, fakeID)).resolves.toBe(
      mockData.apiTransactionDetails.data.json
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL(`transactions/${fakeID}/details`),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the list of transactions", () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiTransactions));

    expect(duskAPI.getTransactions(node)).resolves.toStrictEqual(
      mockData.apiTransactions.data.map(transformTransaction)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("transactions"),
      apiGetOptions
    );
  });

  it("should return a rejected promise, with the original Response in the error's `cause` property, for a 4xx error", () => {
    /**
     * @template T
     * @typedef {{[K in keyof T]: T[K] extends Function ? K : never}[keyof T]} Methods<T>
     */

    const apiMethods = /** @type {Methods<typeof import("..").duskAPI>[]} */ (
      Object.keys(duskAPI).filter((k) => typeof k === "function")
    );

    apiMethods.forEach((method) => {
      const notFoundResponse = new Response("", { status: 404 });

      fetchSpy.mockResolvedValueOnce(notFoundResponse);

      expect(() => duskAPI[method]("foo/bar", "some-id")).rejects.toThrow(
        expect.objectContaining({
          cause: notFoundResponse,
        })
      );
    });
  });

  it("should be able to make the correct request whether the endpoint in env vars ends with a trailing slash or not", () => {
    const expectedURL = new URL(`http://example.com/blocks?node=${node}`);

    fetchSpy
      .mockResolvedValueOnce(makeOKResponse(mockData.apiBlocks))
      .mockResolvedValueOnce(makeOKResponse(mockData.apiBlocks));

    vi.stubEnv(endpointEnvName, "http://example.com");

    duskAPI.getBlocks(node);

    vi.stubEnv(endpointEnvName, "http://example.com/");

    duskAPI.getBlocks(node);

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy).toHaveBeenNthCalledWith(1, expectedURL, apiGetOptions);
    expect(fetchSpy).toHaveBeenNthCalledWith(2, expectedURL, apiGetOptions);

    vi.unstubAllEnvs();
  });

  it("should expose a method to search for blocks and transactions", () => {
    fetchSpy.mockResolvedValueOnce(
      makeOKResponse(mockData.apiSearchBlockResult)
    );

    const query = "some search string";

    expect(duskAPI.search(node, query)).resolves.toStrictEqual(
      transformSearchResult(mockData.apiSearchBlockResult)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      new URL(
        `${import.meta.env[endpointEnvName]}/search/${encodeURIComponent(query)}?node=${node}`
      ),
      apiGetOptions
    );
  });
});
