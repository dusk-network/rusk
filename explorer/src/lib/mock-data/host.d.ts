type HostProvisioner = {
  amount: number;
  eligibility: number;
  faults: number;
  hard_faults: number;
  key: string;
  locked_amt: number;
  owner: { Account: string } | { Contract: string };
  reward: number;
};
