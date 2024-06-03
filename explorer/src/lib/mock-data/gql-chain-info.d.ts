type GQLBlock = {
  fees: number;
  gasSpent: number;
  header: {
    gasLimit: number;
    hash: string;
    height: number;
    nextBlockHash?: string;
    prevBlockHash: string;
    seed: string;
    stateHash: string;
    timestamp: number;
  };
  reward: number;
  transactions: GQLTransaction[];
};

type GQLCallData = {
  contractId: string;
  fnName: string;
};

type GQLChainInfo = {
  blocks: GQLBlock[];
  transactions: GQLTransaction[];
};

type GQLSearchResult = {
  block: {
    header: {
      hash: string;
    };
  } | null;
  transaction?: {
    id: string;
  } | null;
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
