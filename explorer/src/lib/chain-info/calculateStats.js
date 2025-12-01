import { always, condition, filterWith, partition } from "lamb";

import { arraySumByKey } from "$lib/dusk/array";

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
 * @returns {Stats}
 */
function calculateStats(provisioners, lastHeight) {
  const [activeProvisioners, waitingProvisioners] = partition(
    getValidProvisioners(provisioners),
    (p) => p.eligibility <= lastHeight
  );

  return {
    activeProvisioners: activeProvisioners.length,
    activeStake: sumByAmount(activeProvisioners),
    lastBlock: lastHeight,
    waitingProvisioners: waitingProvisioners.length,
    waitingStake: sumByAmount(waitingProvisioners),
  };
}

export default calculateStats;
