type Stats = {
  activeProvisioners: number;
  activeStake: number;
  lastBlock: number;
  txs100blocks: {
    failed: number;
    transfers: number;
  };
  waitingProvisioners: number;
  waitingStake: number;
};
