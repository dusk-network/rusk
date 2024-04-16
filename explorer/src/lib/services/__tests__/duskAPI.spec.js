import { afterAll, afterEach, describe, expect, it, vi } from "vitest";

import { duskAPI } from "..";

describe("duskAPI", () => {
  const fetchSpy = vi.spyOn(global, "fetch");
  const response = new Response("{}", { status: 200 });
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

  fetchSpy.mockResolvedValue(response);

  afterEach(() => {
    fetchSpy.mockClear();
  });

  afterAll(() => {
    fetchSpy.mockRestore();
  });

  it("should expose a method to retrieve a single block", () => {
    duskAPI.getBlock(node, fakeID);

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL(`blocks/${fakeID}`),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the list of blocks", () => {
    duskAPI.getBlocks(node);

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("blocks"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the latest chain info", () => {
    duskAPI.getLatestChainInfo(node);

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("latest"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the market data", () => {
    duskAPI.getMarketData();

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
    duskAPI.getNodeLocations(node);

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("locations"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the statistics", () => {
    duskAPI.getStats(node);

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL("stats"),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve a single transaction", () => {
    duskAPI.getTransaction(node, fakeID);

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL(`transactions/${fakeID}`),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the details of a single transaction", () => {
    duskAPI.getTransactionDetails(node, fakeID);

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      getExpectedURL(`transactions/${fakeID}/details`),
      apiGetOptions
    );
  });

  it("should expose a method to retrieve the list of transactions", () => {
    duskAPI.getTransactions(node);

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

    vi.stubEnv(endpointEnvName, "http://example.com");

    duskAPI.getBlocks(node);

    vi.stubEnv(endpointEnvName, "http://example.com/");

    duskAPI.getBlocks(node);

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy).toHaveBeenNthCalledWith(1, expectedURL, apiGetOptions);
    expect(fetchSpy).toHaveBeenNthCalledWith(2, expectedURL, apiGetOptions);

    vi.unstubAllEnvs();
  });
});
