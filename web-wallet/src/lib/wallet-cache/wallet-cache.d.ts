type WalletCacheBalanceInfo = {
  address: string;
  balance: {
    shielded: AddressBalance;
    unshielded: AccountBalance;
  };
};

type WalletCacheNote = {
  address: string;
  note: Uint8Array;
  nullifier: Uint8Array;
};

type WalletCacheDbNote = Omit<WalletCacheNote, "note" | "nullifier"> & {
  note: ArrayBuffer;
  nullifier: ArrayBuffer;
};

type WalletCacheGetDataType<T extends WalletCacheTableName> =
  T extends "balancesInfo"
    ? WalletCacheBalanceInfo[]
    : T extends "pendingNotesInfo"
      ? WalletCacheDbPendingNoteInfo[]
      : T extends "syncInfo"
        ? WalletCacheSyncInfo[]
        : WalletCacheDbNote[];

type WalletCacheGetEntriesReturnType<
  T extends WalletCacheTableName,
  U extends boolean,
> = U extends false
  ? WalletCacheGetDataType<T>
  : T extends "syncInfo"
    ? never
    : T extends "balancesInfo"
      ? never
      : ArrayBuffer[];

type WalletCacheHistoryEntry = {
  history: Transaction[];
  lastBlockHeight: number;
  psk: string;
};

type WalletCacheNotesMap = Map<string, Map<Uint8Array, Uint8Array>>;

type WalletCachePendingNoteInfo = {
  nullifier: Uint8Array;
  txId: string;
};

type WalletCacheDbPendingNoteInfo = Omit<
  WalletCachePendingNoteInfo,
  "nullifier"
> & {
  nullifier: ArrayBuffer;
};

type WalletCacheSyncInfo = {
  blockHeight: bigint;
  bookmark: bigint;
};

type WalletCacheTableName =
  | "balancesInfo"
  | "pendingNotesInfo"
  | "syncInfo"
  | "spentNotes"
  | "unspentNotes";
