import { unixTsToDate } from "$lib/dusk/date";

/** @type {(v: GQLTransaction) => Transaction} */
const transformTransaction = (tx) => {
  const payloadString = tx.tx.json ?? "{}";
  const payload = JSON.parse(payloadString);

  /** @type {Transaction} */
  const transaction = {
    blockhash: tx.blockHash,
    blockheight: tx.blockHeight,
    date: unixTsToDate(tx.blockTimestamp),
    feepaid: tx.gasSpent * tx.tx.gasPrice,
    gaslimit: tx.tx.gasLimit,
    gasprice: tx.tx.gasPrice,
    gasspent: tx.gasSpent,
    memo: tx.tx.memo ?? "",
    method: tx.tx.isDeploy ? "deploy" : (tx.tx.callData?.fnName ?? "transfer"),
    payload,
    success: tx.err === null,
    txerror: tx.err ?? "",
    txid: tx.id,
    txtype: tx.tx.txType,
  };

  if (payload.value !== undefined) {
    transaction.amount = payload.value;
  }

  if (payload.sender !== undefined) {
    transaction.from = payload.sender;
  }

  if (payload.receiver !== undefined) {
    transaction.to = payload.receiver;
  }

  if (payload.nonce !== undefined) {
    transaction.nonce = payload.nonce;
  }

  return transaction;
};

export default transformTransaction;
