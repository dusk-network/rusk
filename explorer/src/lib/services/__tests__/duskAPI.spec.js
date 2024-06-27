import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { skip, updatePathIn } from "lamb";

import * as mockData from "$lib/mock-data";

import {
  calculateStats,
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

  /** @type {(data: Record<string | number, any> | number) => Response} */
  const makeOKResponse = (data) =>
    new Response(JSON.stringify(data), { status: 200 });

  afterEach(() => {
    fetchSpy.mockClear();
  });

  afterAll(() => {
    fetchSpy.mockRestore();
  });

  it("should expose a method to retrieve a single block", async () => {
    const getByHeightSpy = vi.spyOn(duskAPI, "getBlockHashByHeight");

    fetchSpy
      .mockResolvedValueOnce(
        makeOKResponse(
          updatePathIn(
            mockData.gqlBlock,
            "block.header",
            skip(["nextBlockHash"])
          )
        )
      )
      .mockResolvedValueOnce(
        makeOKResponse({
          block: {
            header: {
              hash: mockData.gqlBlock.block.header.nextBlockHash,
            },
          },
        })
      );

    await expect(duskAPI.getBlock(node, fakeID)).resolves.toStrictEqual(
      transformBlock(mockData.gqlBlock.block)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    \\n\\nfragment TransactionInfo on SpentTransaction {\\n\\tblockHash,\\n\\tblockHeight,\\n\\tblockTimestamp,\\n  err,\\n\\tgasSpent,\\n\\tid,\\n  tx {\\n    callData {\\n      contractId,\\n      data,\\n      fnName\\n    },\\n    gasLimit,\\n    gasPrice,\\n    id\\n  }\\n}\\n\\nfragment BlockInfo on Block {\\n  header {\\n    hash,\\n    gasLimit,\\n    height,\\n    prevBlockHash,\\n    seed,\\n    stateHash,\\n    timestamp,\\n    version\\n  },\\n  fees,\\n  gasSpent,\\n  reward,\\n  transactions {...TransactionInfo}\\n}\\n\\n    query($id: String!) { block(hash: $id) {...BlockInfo} }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""some-id"",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[1][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    query($height: Float!) { block(height: $height) { header { hash } } }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "495869",
        },
        "method": "POST",
      }
    `);
    expect(getByHeightSpy).toHaveBeenCalledTimes(1);
    expect(getByHeightSpy).toHaveBeenCalledWith(
      node,
      mockData.gqlBlock.block.header.height + 1
    );

    getByHeightSpy.mockRestore();
  });

  it("should expose a method to retrieve a block hash by its height", async () => {
    const expectedHash = mockData.gqlBlock.block.header.nextBlockHash;

    fetchSpy
      .mockResolvedValueOnce(
        makeOKResponse({
          block: {
            header: {
              hash: expectedHash,
            },
          },
        })
      )
      .mockResolvedValueOnce(
        makeOKResponse({
          block: null,
        })
      );

    await expect(duskAPI.getBlockHashByHeight(node, 11)).resolves.toBe(
      expectedHash
    );
    await expect(duskAPI.getBlockHashByHeight(node, 11)).resolves.toBe("");
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    query($height: Float!) { block(height: $height) { header { hash } } }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "11",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[1][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    query($height: Float!) { block(height: $height) { header { hash } } }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "11",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the list of blocks", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlBlocks));

    await expect(duskAPI.getBlocks(node, 100)).resolves.toStrictEqual(
      mockData.gqlBlocks.blocks.map(transformBlock)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    \\n\\nfragment TransactionInfo on SpentTransaction {\\n\\tblockHash,\\n\\tblockHeight,\\n\\tblockTimestamp,\\n  err,\\n\\tgasSpent,\\n\\tid,\\n  tx {\\n    callData {\\n      contractId,\\n      data,\\n      fnName\\n    },\\n    gasLimit,\\n    gasPrice,\\n    id\\n  }\\n}\\n\\nfragment BlockInfo on Block {\\n  header {\\n    hash,\\n    gasLimit,\\n    height,\\n    prevBlockHash,\\n    seed,\\n    stateHash,\\n    timestamp,\\n    version\\n  },\\n  fees,\\n  gasSpent,\\n  reward,\\n  transactions {...TransactionInfo}\\n}\\n\\n    query($amount: Int!) { blocks(last: $amount) {...BlockInfo} }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-amount": "100",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the latest chain info", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlLatestChainInfo));

    await expect(duskAPI.getLatestChainInfo(node, 15)).resolves.toStrictEqual({
      blocks: mockData.gqlLatestChainInfo.blocks.map(transformBlock),
      transactions:
        mockData.gqlLatestChainInfo.transactions.map(transformTransaction),
    });
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    \\n\\nfragment TransactionInfo on SpentTransaction {\\n\\tblockHash,\\n\\tblockHeight,\\n\\tblockTimestamp,\\n  err,\\n\\tgasSpent,\\n\\tid,\\n  tx {\\n    callData {\\n      contractId,\\n      data,\\n      fnName\\n    },\\n    gasLimit,\\n    gasPrice,\\n    id\\n  }\\n}\\n\\nfragment BlockInfo on Block {\\n  header {\\n    hash,\\n    gasLimit,\\n    height,\\n    prevBlockHash,\\n    seed,\\n    stateHash,\\n    timestamp,\\n    version\\n  },\\n  fees,\\n  gasSpent,\\n  reward,\\n  transactions {...TransactionInfo}\\n}\\n\\n    query($amount: Int!) {\\n      blocks(last: $amount) {...BlockInfo},\\n      transactions(last: $amount) {...TransactionInfo}\\n    }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-amount": "15",
        },
        "method": "POST",
      }
    `);
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
        "https://api.coingecko.com/api/v3/coins/dusk-network?community_data=false&developer_data=false&localization=false&market_data=true&sparkline=false&tickers=false"
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
    const lastBlockHeight = 1498332;
    const last100BlocksTxs = {
      blocks: [
        { transactions: [{ err: null }] },
        { transactions: [] },
        { transactions: [{ err: "some-error" }] },
      ],
    };
    const expectedStats = calculateStats(
      mockData.hostProvisioners,
      lastBlockHeight,
      [{ err: null }, { err: "some-error" }]
    );

    fetchSpy
      .mockResolvedValueOnce(makeOKResponse(mockData.hostProvisioners))
      .mockResolvedValueOnce(
        makeOKResponse({ block: { header: { height: lastBlockHeight } } })
      )
      .mockResolvedValueOnce(makeOKResponse(last100BlocksTxs));

    await expect(duskAPI.getStats(node)).resolves.toStrictEqual(expectedStats);

    expect(fetchSpy).toHaveBeenCalledTimes(3);
    expect(fetchSpy.mock.calls[0][0]).toBe(`https://${node}/2/rusk`);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"","topic":"provisioners"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[1][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"query { block(height: -1) { header { height } } }","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[2][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[2][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"query { blocks(last: 100) { transactions { err } } }","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
        },
        "method": "POST",
      }
    `);
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
          "Rusk-gqlvar-id": ""some-id"",
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
          "Rusk-gqlvar-id": ""some-id"",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the list of transactions", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlTransactions));

    await expect(duskAPI.getTransactions(node, 100)).resolves.toStrictEqual(
      mockData.gqlTransactions.transactions.map(transformTransaction)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    \\nfragment TransactionInfo on SpentTransaction {\\n\\tblockHash,\\n\\tblockHeight,\\n\\tblockTimestamp,\\n  err,\\n\\tgasSpent,\\n\\tid,\\n  tx {\\n    callData {\\n      contractId,\\n      data,\\n      fnName\\n    },\\n    gasLimit,\\n    gasPrice,\\n    id\\n  }\\n}\\n\\n    query($amount: Int!) { transactions(last: $amount) {...TransactionInfo} }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-amount": "100",
        },
        "method": "POST",
      }
    `);
  });

  it("should return a rejected promise, with the original Response in the error's `cause` property, for a 4xx error", async () => {
    const apiMethods = Object.keys(duskAPI).filter(
      (k) => typeof k === "function"
    );

    for (const apiMethod of apiMethods) {
      const notFoundResponse = new Response("", { status: 404 });

      fetchSpy.mockResolvedValueOnce(notFoundResponse);

      await expect(() =>
        // @ts-expect-error we don't care of the parameters we pass as the call to fetch is mocked
        duskAPI[apiMethod]("foo/bar", "some-id")
      ).rejects.toThrow(
        expect.objectContaining({
          cause: notFoundResponse,
        })
      );
    }
  });

  it("should be able to make the correct request whether the endpoint in env vars ends with a trailing slash or not", () => {
    const expectedURL = new URL(`http://example.com/locations?node=${node}`);

    fetchSpy
      .mockResolvedValueOnce(makeOKResponse(mockData.apiNodeLocations))
      .mockResolvedValueOnce(makeOKResponse(mockData.apiNodeLocations));

    vi.stubEnv(endpointEnvName, "http://example.com");

    duskAPI.getNodeLocations(node);

    vi.stubEnv(endpointEnvName, "http://example.com/");

    duskAPI.getNodeLocations(node);

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy).toHaveBeenNthCalledWith(1, expectedURL, apiGetOptions);
    expect(fetchSpy).toHaveBeenNthCalledWith(2, expectedURL, apiGetOptions);

    vi.unstubAllEnvs();
  });

  it("should expose a method to search for blocks and transactions", async () => {
    const fakeHash1 = Array(64).fill(1).join("");
    const fakeHash2 = Array(64).fill(2).join("");
    const hashResult = {
      block: {
        header: {
          hash: fakeHash1,
        },
      },
    };
    const heightResult = {
      block: {
        header: {
          hash: fakeHash2,
        },
      },
    };

    fetchSpy
      .mockResolvedValueOnce(makeOKResponse(hashResult))
      .mockResolvedValueOnce(makeOKResponse(heightResult));

    await expect(duskAPI.search(node, fakeHash1)).resolves.toStrictEqual(
      transformSearchResult([hashResult, heightResult])
    );

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    query($id: String!) {\\n      block(hash: $id) { header { hash } },\\n      tx(hash: $id) { id }\\n    }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""1111111111111111111111111111111111111111111111111111111111111111"",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[1][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    query($height: Float!) { block(height: $height) { header { hash } } }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "1.1111111111111112e+63",
        },
        "method": "POST",
      }
    `);
  });

  it("should not perform the height search if the query string doesn't contain only numbers", async () => {
    const entryHash =
      "fda46b4e06cc78542db9c780adbaee83a27fdf917de653e8ac34294cf924dd63";

    fetchSpy.mockResolvedValueOnce(
      makeOKResponse(mockData.gqlSearchPossibleResults[0])
    );

    await expect(duskAPI.search(node, entryHash)).resolves.toStrictEqual(
      transformSearchResult([mockData.gqlSearchPossibleResults[0]])
    );

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    query($id: String!) {\\n      block(hash: $id) { header { hash } },\\n      tx(hash: $id) { id }\\n    }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""fda46b4e06cc78542db9c780adbaee83a27fdf917de653e8ac34294cf924dd63"",
        },
        "method": "POST",
      }
    `);
  });

  it("should not perform the hash search if the query string isn't of 64 characters", async () => {
    const entryHeight = "123456";

    fetchSpy.mockResolvedValueOnce(
      makeOKResponse(mockData.gqlSearchPossibleResults[0])
    );

    await expect(duskAPI.search(node, entryHeight)).resolves.toStrictEqual(
      transformSearchResult([mockData.gqlSearchPossibleResults[0]])
    );

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toBe(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "{"data":"\\n    query($height: Float!) { block(height: $height) { header { hash } } }\\n  ","topic":"gql"}",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "123456",
        },
        "method": "POST",
      }
    `);
  });

  it("should not perform any search at all if the query string doesn't satisfy criteria for both hash and height", async () => {
    await expect(duskAPI.search(node, "abc")).resolves.toStrictEqual([]);

    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
