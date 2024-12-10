/** @type {StakeInfo} */
export default {
  amount: {
    eligibility: 0n,
    locked: 0n,
    get total() {
      return this.value + this.locked;
    },
    value: 1000000000000n,
  },
  faults: 0,
  hardFaults: 0,
  reward: 11022842680864n,
};
