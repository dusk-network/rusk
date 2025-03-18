import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { skip, updatePathIn } from "lamb";

import * as mockData from "$lib/mock-data";

import {
  addCountAndUnique,
  calculateStats,
  transformBlock,
  transformSearchResult,
  transformTransaction,
} from "$lib/chain-info";

import { duskAPI } from "..";

describe("duskAPI", () => {
  const fetchSpy = vi.spyOn(global, "fetch");
  const node = new URL(import.meta.env.VITE_NODE_URL, import.meta.url);
  const fakeID = "some-id";
  const apiGetOptions = {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
      Connection: "Keep-Alive",
    },
    method: "GET",
  };

  /** @type {URL} */
  const gqlExpectedURL = new URL("/on/graphql/query", node);
  const endpointEnvName = "VITE_API_ENDPOINT";

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

    await expect(duskAPI.getBlock(fakeID)).resolves.toStrictEqual(
      transformBlock(mockData.gqlBlock.block)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "fragment TransactionInfo on SpentTransaction { blockHash, blockHeight, blockTimestamp, err, gasSpent, id, tx { callData { contractId, data, fnName }, gasLimit, gasPrice, id, isDeploy, memo, txType } } fragment BlockInfo on Block { header { hash, gasLimit, height, prevBlockHash, seed, stateHash, timestamp, version }, fees, gasSpent, reward, transactions {...TransactionInfo} } query($id: String!) { block(hash: $id) {...BlockInfo} }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""some-id"",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[1][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "query($height: Float!) { block(height: $height) { header { hash } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "495869",
        },
        "method": "POST",
      }
    `);
    expect(getByHeightSpy).toHaveBeenCalledTimes(1);
    expect(getByHeightSpy).toHaveBeenCalledWith(
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

    await expect(duskAPI.getBlockHashByHeight(11)).resolves.toBe(expectedHash);
    await expect(duskAPI.getBlockHashByHeight(11)).resolves.toBe("");
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "query($height: Float!) { block(height: $height) { header { hash } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "11",
        },
        "method": "POST",
      }
    `);
    // expect(fetchSpy.mock.calls[1][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "query($height: Float!) { block(height: $height) { header { hash } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "11",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the details of a single block", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlBlockDetails));

    await expect(duskAPI.getBlockDetails(fakeID)).resolves.toBe(
      mockData.gqlBlockDetails.block.header.json
    );
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "query($id: String!) { block(hash: $id) { header { json } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""some-id"",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the list of blocks", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlBlocks));

    await expect(duskAPI.getBlocks(100)).resolves.toStrictEqual(
      mockData.gqlBlocks.blocks.map(transformBlock)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "fragment TransactionInfo on SpentTransaction { blockHash, blockHeight, blockTimestamp, err, gasSpent, id, tx { callData { contractId, data, fnName }, gasLimit, gasPrice, id, isDeploy, memo, txType } } fragment BlockInfo on Block { header { hash, gasLimit, height, prevBlockHash, seed, stateHash, timestamp, version }, fees, gasSpent, reward, transactions {...TransactionInfo} } query($amount: Int!) { blocks(last: $amount) {...BlockInfo} }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-amount": "100",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the latest chain info", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlLatestChainInfo));

    await expect(duskAPI.getLatestChainInfo(15)).resolves.toStrictEqual({
      blocks: mockData.gqlLatestChainInfo.blocks.map(transformBlock),
      transactions:
        mockData.gqlLatestChainInfo.transactions.map(transformTransaction),
    });
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "fragment TransactionInfo on SpentTransaction { blockHash, blockHeight, blockTimestamp, err, gasSpent, id, tx { callData { contractId, data, fnName }, gasLimit, gasPrice, id, isDeploy, memo, txType } } fragment BlockInfo on Block { header { hash, gasLimit, height, prevBlockHash, seed, stateHash, timestamp, version }, fees, gasSpent, reward, transactions {...TransactionInfo} } query($amount: Int!) { blocks(last: $amount) {...BlockInfo}, transactions(last: $amount) {...TransactionInfo} }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
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

    await expect(duskAPI.getNodeLocations()).resolves.toStrictEqual(
      addCountAndUnique(mockData.apiNodeLocations)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it("should expose a method to retrieve the host provisioners", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.hostProvisioners));

    await expect(duskAPI.getProvisioners()).resolves.toStrictEqual(
      mockData.hostProvisioners
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it("should expose a method to retrieve the node info", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.apiNodeInfo));

    await expect(duskAPI.getNodeInfo()).resolves.toStrictEqual(
      mockData.apiNodeInfo
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
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

    await expect(duskAPI.getStats()).resolves.toStrictEqual(expectedStats);

    expect(fetchSpy).toHaveBeenCalledTimes(3);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(
      new URL(
        `${import.meta.env.VITE_RUSK_PATH || ""}/on/node/provisioners`,
        node.origin
      )
    );
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[1][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "query { block(height: -1) { header { height } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[2][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[2][1]).toMatchInlineSnapshot(`
      {
        "body": "query { blocks(last: 100) { transactions { err } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve a single transaction", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlTransaction));

    await expect(duskAPI.getTransaction(fakeID)).resolves.toStrictEqual(
      transformTransaction(mockData.gqlTransaction.tx)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "fragment TransactionInfo on SpentTransaction { blockHash, blockHeight, blockTimestamp, err, gasSpent, id, tx { callData { contractId, data, fnName }, gasLimit, gasPrice, id, isDeploy, memo, txType } } query($id: String!) { tx(hash: $id) {...TransactionInfo} }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
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

    await expect(duskAPI.getTransactionDetails(fakeID)).resolves.toBe(
      mockData.gqlTransactionDetails.tx.tx.json
    );
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "query($id: String!) { tx(hash: $id) { tx { json } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""some-id"",
        },
        "method": "POST",
      }
    `);
  });

  it("should expose a method to retrieve the list of transactions", async () => {
    fetchSpy.mockResolvedValueOnce(makeOKResponse(mockData.gqlTransactions));

    await expect(duskAPI.getTransactions(100)).resolves.toStrictEqual(
      mockData.gqlTransactions.transactions.map(transformTransaction)
    );
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "fragment TransactionInfo on SpentTransaction { blockHash, blockHeight, blockTimestamp, err, gasSpent, id, tx { callData { contractId, data, fnName }, gasLimit, gasPrice, id, isDeploy, memo, txType } } query($amount: Int!) { transactions(last: $amount) {...TransactionInfo} }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
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
    const expectedURL = new URL(
      "https://nodes.dusk.network/on/network/peers_location"
    );

    fetchSpy
      .mockResolvedValueOnce(makeOKResponse(mockData.apiNodeLocations))
      .mockResolvedValueOnce(makeOKResponse(mockData.apiNodeLocations));

    vi.stubEnv(endpointEnvName, "http://example.com");

    duskAPI.getNodeLocations();

    vi.stubEnv(endpointEnvName, "http://example.com/");

    duskAPI.getNodeLocations();

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy).toHaveBeenNthCalledWith(1, expectedURL, {
      headers: {
        Accept: "application/json",
        "Accept-Charset": "utf-8",
        Connection: "Keep-Alive",
      },
      method: "POST",
    });
    expect(fetchSpy).toHaveBeenNthCalledWith(2, expectedURL, {
      headers: {
        Accept: "application/json",
        "Accept-Charset": "utf-8",
        Connection: "Keep-Alive",
      },
      method: "POST",
    });

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

    await expect(duskAPI.search(fakeHash1)).resolves.toStrictEqual(
      transformSearchResult(hashResult)
    );

    await expect(duskAPI.search(fakeHash2)).resolves.toStrictEqual(
      transformSearchResult(heightResult)
    );

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "query($id: String!) { block(hash: $id) { header { hash } }, tx(hash: $id) { id } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""1111111111111111111111111111111111111111111111111111111111111111"",
        },
        "method": "POST",
      }
    `);
    expect(fetchSpy.mock.calls[1][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][1]).toMatchInlineSnapshot(`
      {
        "body": "query($id: String!) { block(hash: $id) { header { hash } }, tx(hash: $id) { id } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-id": ""2222222222222222222222222222222222222222222222222222222222222222"",
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

    await expect(duskAPI.search(entryHash)).resolves.toStrictEqual(
      transformSearchResult(mockData.gqlSearchPossibleResults[0])
    );

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "query($id: String!) { block(hash: $id) { header { hash } }, tx(hash: $id) { id } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
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

    await expect(duskAPI.search(entryHeight)).resolves.toStrictEqual(
      transformSearchResult(mockData.gqlSearchPossibleResults[0])
    );

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[0][1]).toMatchInlineSnapshot(`
      {
        "body": "query($height: Float!) { block(height: $height) { header { hash } } }",
        "headers": {
          "Accept": "application/json",
          "Accept-Charset": "utf-8",
          "Connection": "Keep-Alive",
          "Content-Type": "application/json",
          "Rusk-gqlvar-height": "123456",
        },
        "method": "POST",
      }
    `);
  });

  it("should not perform any search at all if the query string doesn't satisfy criteria for both hash and height", async () => {
    await expect(duskAPI.search("abc")).resolves.toStrictEqual(null);

    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("should return a mempool message if the transaction is in the mempool", async () => {
    fetchSpy
      .mockResolvedValueOnce(makeOKResponse({ tx: null }))
      .mockResolvedValueOnce(makeOKResponse({ mempoolTx: true }));

    await expect(duskAPI.getTransaction(fakeID)).resolves.toBe(
      "This transaction is currently in the mempool and has not yet been confirmed. The transaction details will be displayed after confirmation."
    );

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][0]).toStrictEqual(gqlExpectedURL);
  });

  it("should throw an error if the transaction is not found", async () => {
    fetchSpy
      .mockResolvedValueOnce(makeOKResponse({ tx: null }))
      .mockResolvedValueOnce(makeOKResponse({ mempoolTx: null }));

    await expect(duskAPI.getTransaction(fakeID)).rejects.toThrow(
      "Transaction not found"
    );

    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy.mock.calls[0][0]).toStrictEqual(gqlExpectedURL);
    expect(fetchSpy.mock.calls[1][0]).toStrictEqual(gqlExpectedURL);
  });
});
