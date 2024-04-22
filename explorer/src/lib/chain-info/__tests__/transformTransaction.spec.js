import { describe, expect, it } from "vitest";
import { skipIn } from "lamb";

import { apiTransaction } from "$lib/mock-data";

import { transformTransaction } from "..";

describe("transformTransaction", () => {
  const txData = apiTransaction.data[0];
  const expectedTx = {
    blockhash:
      "3c6e4018cfa86723e50644e33d3990bc27fc794f6b49fbf6290e4d308e07bd2d",
    blockheight: 487166,
    contract: "Transfer",
    date: new Date(txData.blockts * 1000),
    feepaid: 290766,
    gaslimit: 500000000,
    gasprice: 1,
    gasspent: 290766,
    method: "transfer",
    success: true,
    txerror: "",
    txid: "4877687c2dbf154248d3ddee9ba0d81e3431f39056f82a46819da041d4ac0e04",
  };

  it("should transform a block received from the API into the format used by the Explorer", () => {
    expect(transformTransaction(txData)).toStrictEqual(expectedTx);
  });

  it("should give defaults to optional properties if they are missing", () => {
    const incompleteTx = skipIn(txData, ["method"]);

    expect(transformTransaction(incompleteTx)).toStrictEqual({
      ...expectedTx,
      method: "",
    });
  });
});
