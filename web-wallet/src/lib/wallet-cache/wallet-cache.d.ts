type WalletCacheNote = {
  address: string;
  note: Uint8Array;
  nullifier: Uint8Array;
};

type WalletCacheGetDataType<T extends WalletCacheTableName> =
  T extends "pendingNotesInfo"
    ? WalletCachePendingNoteInfo[]
    : T extends "syncInfo"
      ? WalletCacheSyncInfo[]
      : WalletCacheNote[];

type WalletCacheGetEntriesReturnType<
  T extends WalletCacheTableName,
  U extends boolean,
> = U extends false
  ? WalletCacheGetDataType<T>
  : T extends "syncInfo"
    ? never
    : Uint8Array[];

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

type WalletCacheSyncInfo = {
  blockHeight: bigint;
  bookmark: bigint;
};

type WalletCacheTableName =
  | "pendingNotesInfo"
  | "syncInfo"
  | "spentNotes"
  | "unspentNotes";

type WalletCacheTreasury = {
  address: (
    identifier: import("$lib/vendor/w3sper.js/src/mod").Profile["address"]
  ) => Promise<Map<Uint8Array, Uint8Array>>;
};
