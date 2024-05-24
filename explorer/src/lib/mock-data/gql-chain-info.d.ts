type GQLCallData = {
  contractId: string;
  fnName: string;
};

type GQLTransaction = {
  blockHash: string;
  blockHeight: number;
  blockTimestamp: number;
  err: string | null;
  gasSpent: number;
  id: string;
  tx: {
    callData: GQLCallData | null;
    gasLimit: number;
    gasPrice: number;
    id: string;
  };
};
