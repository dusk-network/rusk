import { map } from "lamb";

import { unixTsToDate } from "$lib/dusk/date";

import { transformTransaction } from ".";

/** @type {(v: GQLBlock) => Block} */
const transformBlock = (v) => ({
  header: {
    date: unixTsToDate(v.header.timestamp),
    feespaid: v.fees,
    hash: v.header.hash,
    height: v.header.height,
    nextblockhash: v.header.nextBlockHash ?? "",
    prevblockhash: v.header.prevBlockHash,
    reward: v.reward,
    seed: v.header.seed,
    statehash: v.header.stateHash,
  },
  transactions: {
    data: map(v.transactions, transformTransaction),
    stats: {
      averageGasPrice: v.gasSpent > 0 ? v.fees / v.gasSpent : 0,
      gasLimit: v.header.gasLimit,
      gasUsed: v.gasSpent,
    },
  },
});

export default transformBlock;
