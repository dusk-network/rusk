type WalletCacheNote = {
  block_height: number;
  note: number[];
  nullifier: number[];
  pos: number;
  psk: string;
};

type WalletCacheHistoryEntry = {
  history: Transaction[];
  lastBlockHeight: number;
  psk: string;
};
