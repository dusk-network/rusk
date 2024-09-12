import { describe, expect, it } from "vitest";

import { gqlTransaction } from "$lib/mock-data";

import { transformTransaction } from "..";

describe("transformTransaction", () => {
  const txData = gqlTransaction.tx;
  const expectedTx = {
    blockhash:
      "3c6e4018cfa86723e50644e33d3990bc27fc794f6b49fbf6290e4d308e07bd2d",
    blockheight: 487166,
    date: new Date(txData.blockTimestamp * 1000),
    feepaid: 290766,
    gaslimit: 500000000,
    gasprice: 1,
    gasspent: 290766,
    memo: gqlTransaction.tx.tx.memo,
    method: "transfer",
    success: true,
    txerror: "",
    txid: "4877687c2dbf154248d3ddee9ba0d81e3431f39056f82a46819da041d4ac0e04",
  };

  it("should transform a transaction received from the GraphQL API into the format used by the Explorer", () => {
    expect(transformTransaction(txData)).toStrictEqual(expectedTx);
  });

  it("should use the call data if present to set the method and contract name", () => {
    const data = {
      ...txData,
      tx: {
        ...txData.tx,
        callData: {
          contractId:
            "0200000000000000000000000000000000000000000000000000000000000000",
          fnName: "stake",
        },
      },
    };
    const expected = {
      ...expectedTx,
      method: "stake",
    };

    expect(transformTransaction(data)).toStrictEqual(expected);
  });

  it('should use "deploy" as method, if the related property is `true`, regardless of the `callData.fnName` value', () => {
    const dataA = {
      ...txData,
      tx: {
        ...txData.tx,
        callData: {
          contractId:
            "0200000000000000000000000000000000000000000000000000000000000000",
          fnName: "transfer",
        },
        isDeploy: true,
      },
    };
    const dataB = {
      ...txData,
      tx: {
        ...txData.tx,
        isDeploy: true,
      },
    };
    const expected = {
      ...expectedTx,
      method: "deploy",
    };

    expect(transformTransaction(dataA)).toStrictEqual(expected);
    expect(transformTransaction(dataB)).toStrictEqual(expected);
  });

  it("should set the success property to `false` if the an error is present and use the message in the `txerror` property", () => {
    const data = {
      ...txData,
      err: "Some error message",
    };
    const expected = {
      ...expectedTx,
      success: false,
      txerror: data.err,
    };

    expect(transformTransaction(data)).toStrictEqual(expected);
  });
});
