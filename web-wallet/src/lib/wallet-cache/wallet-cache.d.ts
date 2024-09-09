type WalletCacheNote = {
  block_height: number;
  note: number[];
  nullifier: number[];
  pos: number;
  psk: string;
};

type WalletCacheTransaction = {
  amount: number;
  block_height: number;
  direction: "In" | "Out";
  fee: number;
  id: string;
  tx_type: string;
};

type WalletCacheHistoryEntry = {
  history: WalletCacheTransaction[];
  lastBlockHeight: number;
  psk: string;
};
