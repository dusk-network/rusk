type APIBlockHeader = {
  __typename: string;
  feespaid: number;
  hash: string;
  height: number;
  nextblockhash?: string;
  prevblockhash?: string;
  reward: number;
  seed: string;
  statehash?: string;
  timestamp: string;
  ts: number;
  version: string;
};

type APIBlockTransactions = {
  data: APITransaction[];
  stats: {
    averageGasPrice: number;
    gasLimit: number;
    gasUsed: number;
  };
};

type APIBlock = {
  __typename: string;
  header: APIBlockHeader;
  transactions: APIBlockTransactions;
};

type APIChainInfo = {
  blocks: APIBlock[];
  transactions: APITransaction[];
};

type APISearchBlockResult = {
  data: {
    data: {
      blocks: {
        header: { hash: string };
      }[];
    };
  };
};

type APISearchNoResult = {
  data: {};
};

type APISearchTransactionResult = {
  data: {
    data: {
      transactions: {
        __typename: string;
        tx: { id: string };
        txid: string;
      }[];
    };
  };
};

type APISearchResult =
  | APISearchBlockResult
  | APISearchNoResult
  | APISearchTransactionResult;

type APITransaction = {
  __typename: string;
  blockhash: string;
  blockheight: number;
  blocktimestamp: string;
  blockts: number;
  contract: string;
  feepaid: number;
  gaslimit: number;
  gasprice: number;
  gasspent: number;
  method?: string;
  success: boolean;
  txerror: string;
  txid: string;
  txtype: string;
};
