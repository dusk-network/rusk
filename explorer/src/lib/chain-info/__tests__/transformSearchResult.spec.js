import { describe, expect, it } from "vitest";

import {
  apiSearchBlockResult,
  apiSearchNoResult,
  apiSearchTransactionResult,
} from "$lib/mock-data";

import { transformSearchResult } from "..";

describe("transformSearchResult", () => {
  it("should transform an API result containing a block into an application search result", () => {
    expect(transformSearchResult(apiSearchBlockResult)).toStrictEqual([
      {
        id: apiSearchBlockResult.data.data.blocks[0].header.hash,
        type: "block",
      },
    ]);
  });

  it("should transform an API result containing a transaction into an application search result", () => {
    expect(transformSearchResult(apiSearchTransactionResult)).toStrictEqual([
      {
        id: apiSearchTransactionResult.data.data.transactions[0].txid,
        type: "transaction",
      },
    ]);
  });

  it("should transform an API result containing no data into an application search result", () => {
    expect(transformSearchResult(apiSearchNoResult)).toStrictEqual([]);
  });

  /*
   * This is not a real situation with the current API, but the
   * transform function is able to handle it.
   */
  it("should transform an API result containing multiple blocks and transactions into an application search result", () => {
    const apiResult = {
      data: {
        data: {
          blocks: [
            apiSearchBlockResult.data.data.blocks[0],
            apiSearchBlockResult.data.data.blocks[0],
          ],
          transactions: [
            apiSearchTransactionResult.data.data.transactions[0],
            apiSearchTransactionResult.data.data.transactions[0],
          ],
        },
      },
    };

    expect(transformSearchResult(apiResult)).toStrictEqual([
      {
        id: apiSearchBlockResult.data.data.blocks[0].header.hash,
        type: "block",
      },
      {
        id: apiSearchBlockResult.data.data.blocks[0].header.hash,
        type: "block",
      },
      {
        id: apiSearchTransactionResult.data.data.transactions[0].txid,
        type: "transaction",
      },
      {
        id: apiSearchTransactionResult.data.data.transactions[0].txid,
        type: "transaction",
      },
    ]);
  });
});
