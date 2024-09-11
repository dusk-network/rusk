import { unixTsToDate } from "$lib/dusk/date";

/** @param {string} [s] */
const capitalize = (s) => (s ? `${s[0].toUpperCase()}${s.slice(1)}` : "");

/** @type {(v: GQLTransaction) => Transaction} */
const transformTransaction = (tx) => ({
  blockhash: tx.blockHash,
  blockheight: tx.blockHeight,
  contract: tx.tx.callData ? capitalize(tx.tx.callData.fnName) : "Transfer",
  date: unixTsToDate(tx.blockTimestamp),
  feepaid: tx.gasSpent * tx.tx.gasPrice,
  gaslimit: tx.tx.gasLimit,
  gasprice: tx.tx.gasPrice,
  gasspent: tx.gasSpent,
  method: tx.tx.callData?.fnName ?? "transfer",
  success: tx.err === null,
  txerror: tx.err ?? "",
  txid: tx.id,
});

export default transformTransaction;
