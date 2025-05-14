import { describe, expect, it } from "vitest";

import { gqlTransaction } from "$lib/mock-data";

import { transformTransaction } from "..";

describe("transformTransaction", () => {
  const tx = gqlTransaction.tx;
  const expectedTx = {
    amount: 9812378912731,
    blockhash:
      "3c6e4018cfa86723e50644e33d3990bc27fc794f6b49fbf6290e4d308e07bd2d",
    blockheight: 487166,
    date: new Date(tx.blockTimestamp * 1000),
    feepaid: 290766,
    from: "kT5BamV2Xq6JpnoCq9ux7EmYRShdg4W1sLzyGFUqicmZTqxeMWLiTGpBWNhpjiPtKqvWFFmdxVBAjts3oEnZgPfuiyTRwDdASJQjTSy7wNhHdCeKtzExREcXZsmmMRLwKbXj",
    gaslimit: 500000000,
    gasprice: 1,
    gasspent: 290766,
    memo: tx.tx.memo,
    method: "transfer",
    nonce: 156,
    payload: {
      call: null,
      deposit: 0,
      fee: {
        // eslint-disable-next-line camelcase
        gas_limit: "2500000",
        // eslint-disable-next-line camelcase
        gas_price: "1",
        // eslint-disable-next-line camelcase
        refund_address:
          "kT5BamV2Xq6JpnoCq9ux7EmYRShdg4W1sLzyGFUqicmZTqxeMWLiTGpBWNhpjiPtKqvWFFmdxVBAjts3oEnZgPfuiyTRwDdASJQjTSy7wNhHdCeKtzExREcXZsmmMRLwKbXj",
      },
      // eslint-disable-next-line camelcase
      is_deploy: false,
      memo: null,
      nonce: 156,
      receiver:
        "mHWZo9qYUhqp2SEmtWN7EuDKFwVrjgdEyzpHdfLB6R9scRAq5EkUQyLB9fBfCGt1wjxfEpFxHq9MBPVVPY3Lk3JKnQLWZzj7UYAR4mGmeGQZwCaeCS8uA63ZPKnGpjiUnj",
      sender:
        "kT5BamV2Xq6JpnoCq9ux7EmYRShdg4W1sLzyGFUqicmZTqxeMWLiTGpBWNhpjiPtKqvWFFmdxVBAjts3oEnZgPfuiyTRwDdASJQjTSy7wNhHdCeKtzExREcXZsmmMRLwKbXj",
      type: "moonlight",
      value: 9812378912731,
    },

    success: true,
    to: "mHWZo9qYUhqp2SEmtWN7EuDKFwVrjgdEyzpHdfLB6R9scRAq5EkUQyLB9fBfCGt1wjxfEpFxHq9MBPVVPY3Lk3JKnQLWZzj7UYAR4mGmeGQZwCaeCS8uA63ZPKnGpjiUnj",
    txerror: "",
    txid: "4877687c2dbf154248d3ddee9ba0d81e3431f39056f82a46819da041d4ac0e04",
    txtype: "Moonlight",
  };

  it("should transform a transaction received from the GraphQL API into the format used by the Explorer", () => {
    expect(transformTransaction(tx)).toStrictEqual(expectedTx);
  });

  it("should use the call data if present to set the method and contract name", () => {
    const data = {
      ...tx,
      tx: {
        ...tx.tx,
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
      ...tx,
      tx: {
        ...tx.tx,
        callData: {
          contractId:
            "0200000000000000000000000000000000000000000000000000000000000000",
          fnName: "transfer",
        },
        isDeploy: true,
      },
    };
    const dataB = {
      ...tx,
      tx: {
        ...tx.tx,
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
      ...tx,
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
