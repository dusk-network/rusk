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
  T extends "pendingNotesInfo"
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
  | "pendingNotesInfo"
  | "syncInfo"
  | "spentNotes"
  | "unspentNotes";

type WalletCacheTreasury = {
  address: (
    identifier: import("$lib/vendor/w3sper.js/src/mod").Profile["address"]
  ) => Promise<Map<Uint8Array, Uint8Array>>;
};
