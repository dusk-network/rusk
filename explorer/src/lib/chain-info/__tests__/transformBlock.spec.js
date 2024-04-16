import { describe, expect, it } from "vitest";
import { skipIn } from "lamb";

import { apiBlock } from "$lib/mock-data";

import { transformBlock } from "..";

describe("transformBlock", () => {
  const blockData = apiBlock.data.blocks[0];
  const expectedBlock = {
    header: {
      date: new Date(blockData.header.ts * 1000),
      feespaid: 580718,
      hash: "bd5c99bb720b03500e89f103fe66113ba62f2e124ed9651563f38fd15977719f",
      height: 495868,
      nextblockhash:
        "6011556208a85e6001bd01ccbf936486b91318a7f6cbcf7ab810adf6fae34204",
      prevblockhash:
        "07b74b35c2c7cf8f41426cd0870bafa1a2c7adee3fdd876643548096186fc4cb",
      reward: 16000000000,
      seed: "af15447e3a004a79d4ae8b084f7b76b78d95880bb63e1cfa79250a310731f52e6d84ee42a5d6fc2cb99c5b1f489761f6",
      statehash:
        "20bb0a677b93f084afadfd34bec3ac3feee33a020b81d9549afa2268e8543acb",
      version: "0",
    },
    transactions: {
      data: [
        {
          blockhash:
            "bd5c99bb720b03500e89f103fe66113ba62f2e124ed9651563f38fd15977719f",
          blockheight: 495868,
          contract: "Transfer",
          date: new Date(blockData.transactions.data[0].blockts * 1000),
          feepaid: 290866,
          gaslimit: 500000000,
          gasprice: 1,
          gasspent: 290866,
          method: "transfer",
          success: true,
          txerror: "",
          txid: "3a3f6f90a1012ae751b4448bcb8e98def0ba2b18170239bd69fcf8e2e37f0602",
        },
        {
          blockhash:
            "bd5c99bb720b03500e89f103fe66113ba62f2e124ed9651563f38fd15977719f",
          blockheight: 495868,
          contract: "Transfer",
          date: new Date(blockData.transactions.data[1].blockts * 1000),
          feepaid: 289852,
          gaslimit: 500000000,
          gasprice: 1,
          gasspent: 289852,
          method: "transfer",
          success: true,
          txerror: "",
          txid: "07bfabea1d94c16f2dc3697fa642f6cecea6e81bf76b9644efbb6e2723b76d00",
        },
      ],
      stats: { averageGasPrice: 1, gasLimit: 5000000000, gasUsed: 580718 },
    },
  };

  it("should transform a block received from the API into the format used by the Explorer", () => {
    expect(transformBlock(blockData)).toStrictEqual(expectedBlock);
  });

  it("should give defaults to optional properties if they are missing", () => {
    const incompleteBlockHeader = skipIn(blockData.header, [
      "nextblockhash",
      "prevblockhash",
      "statehash",
    ]);
    const incompleteBlock = {
      ...blockData,
      header: incompleteBlockHeader,
    };

    expect(transformBlock(incompleteBlock)).toStrictEqual({
      ...expectedBlock,
      header: {
        ...expectedBlock.header,
        nextblockhash: "",
        prevblockhash: "",
        statehash: "",
      },
    });
  });
});
