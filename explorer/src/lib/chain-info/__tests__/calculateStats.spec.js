import { describe, expect, it } from "vitest";

import { hostProvisioners } from "$lib/mock-data";

import { calculateStats } from "..";

describe("calculateStats", () => {
  const lastBlockHeight = 1498332;

  it("should calculate the stats with the given parameters", () => {
    const last100BlocksTxs = [
      { err: null },
      { err: "some error" },
      { err: null },
      { err: "some other error" },
      { err: null },
    ];
    const expectedStats = {
      activeProvisioners: 944,
      activeStake: 58872472691778710,
      lastBlock: 1498332,
      txs100blocks: { failed: 2, transfers: 5 },
      waitingProvisioners: 82,
      waitingStake: 4381569737564303,
    };
    expect(
      calculateStats(hostProvisioners, lastBlockHeight, last100BlocksTxs)
    ).toStrictEqual(expectedStats);
  });

  it("should accept an empty array of provisioners and transactions", () => {
    const expectedStats = {
      activeProvisioners: 0,
      activeStake: 0,
      lastBlock: 1498332,
      txs100blocks: { failed: 0, transfers: 0 },
      waitingProvisioners: 0,
      waitingStake: 0,
    };

    expect(calculateStats([], lastBlockHeight, [])).toStrictEqual(
      expectedStats
    );
  });
});
