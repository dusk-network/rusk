type BlockHeader = {
  date: Date;
  feespaid: number;
  hash: string;
  height: number;
  nextblockhash: string;
  prevblockhash: string;
  reward: number;
  seed: string;
  statehash: string;
};

type BlockTransactions = {
  data: Transaction[];
  stats: {
    averageGasPrice: number;
    gasLimit: number;
    gasUsed: number;
  };
};

type Block = {
  header: BlockHeader;
  transactions: BlockTransactions;
};

type ChainInfo = {
  blocks: Block[];
  transactions: Transaction[];
};

type SearchResult = {
  id: string;
  type: "block" | "transaction";
};

type Transaction = {
  blockhash: string;
  blockheight: number;
  contract: string;
  date: Date;
  feepaid: number;
  gaslimit: number;
  gasprice: number;
  gasspent: number;
  method: string;
  success: boolean;
  txerror: string;
  txid: string;
};
