import { describe, expect, it } from "vitest";

import { hostProvisioners } from "$lib/mock-data";

import { calculateStats } from "..";

describe("calculateStats", () => {
  const lastBlockHeight = 1498332;

  it("should calculate the stats with the given parameters", () => {
    const expectedStats = {
      activeProvisioners: 213,
      activeStake: 56732778800000000,
      lastBlock: 1498332,
      waitingProvisioners: 0,
      waitingStake: 0,
    };
    expect(calculateStats(hostProvisioners, lastBlockHeight)).toStrictEqual(
      expectedStats
    );
  });

  it("should accept an empty array of provisioners and transactions", () => {
    const expectedStats = {
      activeProvisioners: 0,
      activeStake: 0,
      lastBlock: 1498332,
      waitingProvisioners: 0,
      waitingStake: 0,
    };

    expect(calculateStats([], lastBlockHeight)).toStrictEqual(expectedStats);
  });
});
