import { afterAll, afterEach, describe, expect, it, vi } from "vitest";

import * as mockData from "$lib/mock-data";

import {
  transformAPITransaction,
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

  /** @type {string} */
  const gqlExpectedURL = `https://${node}/02/Chain`;

  const endpointEnvName = "VITE_API_ENDPOINT";

  /** @type {(endpoint: string) => URL} */
  const getAPIExpectedURL = (endpoint) =>
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

  it("should expose a method to retrieve a single block", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiBlock));

    await expect(duskAPI.getBlock(node, fakeID)).resolves.toStrictEqual(
      transformBlock(mockData.apiBlock.data.blocks[0])
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getAPIExpectedURL(`blocks/${fakeID}`),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the list of blocks", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiBlocks));

    await expect(duskAPI.getBlocks(node)).resolves.toStrictEqual(
      mockData.apiBlocks.data.blocks.map(transformBlock)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getAPIExpectedURL("blocks"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the latest chain info", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiLatestChainInfo));

    await expect(duskAPI.getLatestChainInfo(node)).resolves.toStrictEqual({
      blocks: mockData.apiLatestChainInfo.data.blocks.map(transformBlock),
      transactions: mockData.apiLatestChainInfo.data.transactions.map(
        transformAPITransaction
      ),
    });
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getAPIExpectedURL("latest"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the market data", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiMarketData));

    await expect(duskAPI.getMarketData()).resolves.toStrictEqual({
      currentPrice: mockData.apiMarketData.market_data.current_price,
      marketCap: mockData.apiMarketData.market_data.market_cap,
    });
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      new URL(
        getAPIExpectedURL("quote")
          .toString()
          .replace(/(\?).+$/, "$1")
      ),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the node locations", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiNodeLocations));

    await expect(duskAPI.getNodeLocations(node)).resolves.toStrictEqual(
      mockData.apiNodeLocations.data
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getAPIExpectedURL("locations"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the statistics", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiStats));

    await expect(duskAPI.getStats(node)).resolves.toStrictEqual(
      mockData.apiStats
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getAPIExpectedURL("stats"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve a single transaction", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlTransaction));

    await expect(duskAPI.getTransaction(node, fakeID)).resolves.toStrictEqual(
      transformTransaction(mockData.gqlTransaction.tx)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    \\nfragment TransactionInfo on SpentTransaction {\\n\\tblockHash,\\n\\tblockHeight,\\n\\tblockTimestamp,\\n  err,\\n\\tgasSpent,\\n\\tid,\\n  tx {\\n    callData {\\n      contractId,\\n      data,\\n      fnName\\n    },\\n    gasLimit,\\n    gasPrice,\\n    id\\n  }\\n}\\n\\n    query($id: String!) { tx(hash: $id) {...TransactionInfo} }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": "some-id",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the details of a single transaction", async () => {
    fetchSpy.mockResolvedValueOnce(
      makeOKResponse(mockData.gqlTransactionDetails)
    );

    await expect(duskAPI.getTransactionDetails(node, fakeID)).resolves.toBe(
      mockData.gqlTransactionDetails.tx.raw
    );
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"query($id: String!) { tx(hash: $id) { raw } }","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": "some-id",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the list of transactions", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiTransactions));

    await expect(duskAPI.getTransactions(node)).resolves.toStrictEqual(
      mockData.apiTransactions.data.map(transformAPITransaction)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getAPIExpectedURL("transactions"),
      apiGetOptions
    );
  });

  it("should return a rejected promise, with the original Response in the error's `cause` property, for a 4xx error", async () => {
    /**
     * @template T
     * @typedef {{[K in keyof T]: T[K] extends Function ? K : never}[keyof T]} Methods<T>
     */

    const apiMethods = /** @type {Methods<typeof import("..").duskAPI>[]} */ (
      Object.keys(duskAPI).filter((k) => typeof k === "function")
    );

    for (const apiMethod of apiMethods) {
      const notFoundResponse = new Response("", { status: 404 });

      fetchSpy.mockResolvedValueOnce(notFoundResponse);

      await expect(() =>
        duskAPI[apiMethod]("foo/bar", "some-id")
      ).rejects.toThrow(
        expect.objectContaining({
          cause: notFoundResponse,
        })
      );
    }
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

  it("should expose a method to search for blocks and transactions", async () => {
    fetchSpy.mockResolvedValueOnce(
      makeOKResponse(mockData.apiSearchBlockResult)
    );

    const query = "some search string";

    await expect(duskAPI.search(node, query)).resolves.toStrictEqual(
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
