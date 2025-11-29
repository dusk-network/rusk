type Stats = {
  activeProvisioners: number;
  activeStake: number;
  lastBlock: number;
  waitingProvisioners: number;
  waitingStake: number;
  txCount?: {
    public: number;
    shielded: number;
    total: number;
  };
};
