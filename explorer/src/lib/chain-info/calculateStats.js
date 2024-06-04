import {
  always,
  compose,
  condition,
  filterWith,
  getKey,
  partition,
} from "lamb";

import { arraySumByKey } from "$lib/dusk/array";

/** @type {(txs: Pick<GQLTransaction, "err">[]) => number} */
const getFailedTxAmount = compose(
  getKey("length"),
  filterWith((tx) => tx.err !== null)
);

/**
 * We take into account only provisioners with
 * the minimum stake amount of 1000 Dusk (1e12 Lux).
 *
 * @type {(provisioners: HostProvisioner[]) => HostProvisioner[]}
 */
const getValidProvisioners = filterWith((p) => p.amount >= 1e12);

/**
 * Sums the values of the "amount" key in the received array.
 * Returns zero if the array is empty.
 */
const sumByAmount = condition(
  (provisioners) => provisioners.length > 0,
  arraySumByKey("amount"),
  always(0)
);

/**
 * @param {HostProvisioner[]} provisioners
 * @param {number} lastHeight
 * @param {Pick<GQLTransaction, "err">[]} last100BlocksTxs
 * @returns {Stats}
 */
function calculateStats(provisioners, lastHeight, last100BlocksTxs) {
  const [activeProvisioners, waitingProvisioners] = partition(
    getValidProvisioners(provisioners),
    (p) => p.eligibility <= lastHeight
  );

  return {
    activeProvisioners: activeProvisioners.length,
    activeStake: sumByAmount(activeProvisioners),
    lastBlock: lastHeight,
    txs100blocks: {
      failed: getFailedTxAmount(last100BlocksTxs),
      transfers: last100BlocksTxs.length,
    },
    waitingProvisioners: waitingProvisioners.length,
    waitingStake: sumByAmount(waitingProvisioners),
  };
}

export default calculateStats;
