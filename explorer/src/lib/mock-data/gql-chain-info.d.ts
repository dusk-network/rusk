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
  block?: {
    header: {
      hash: string;
    };
  } | null;
  tx?: {
    id: string;
  } | null;
  account?: {
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
    blobHashes: string[] | null;
    callData: GQLCallData | null;
    gasLimit: number;
    gasPrice: number;
    id: string;
    isDeploy: boolean;
    memo: string;
    txType: string;
    json?: string;
  };
};
