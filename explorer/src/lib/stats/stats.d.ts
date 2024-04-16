type Stats = {
  activeProvisioners: number;
  activeStake: number;
  hostlist: number;
  lastBlock: number;
  tps: number;
  txs100blocks: {
    failed: number;
    transfers: number;
  };
  waitingProvisioners: number;
  waitingStake: number;
};
