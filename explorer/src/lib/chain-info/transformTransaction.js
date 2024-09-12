import { unixTsToDate } from "$lib/dusk/date";

/** @type {(v: GQLTransaction) => Transaction} */
const transformTransaction = (tx) => ({
  blockhash: tx.blockHash,
  blockheight: tx.blockHeight,
  date: unixTsToDate(tx.blockTimestamp),
  feepaid: tx.gasSpent * tx.tx.gasPrice,
  gaslimit: tx.tx.gasLimit,
  gasprice: tx.tx.gasPrice,
  gasspent: tx.gasSpent,
  memo: tx.tx.memo ?? "",
  method: tx.tx.isDeploy ? "deploy" : tx.tx.callData?.fnName ?? "transfer",
  success: tx.err === null,
  txerror: tx.err ?? "",
  txid: tx.id,
});

export default transformTransaction;
