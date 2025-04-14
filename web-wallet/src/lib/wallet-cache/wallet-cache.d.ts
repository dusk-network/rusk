/**
 * Sync info coming from the unspent
 * notes stream, enriched with the
 * block hash.
 */
type NotesSyncInfo = {
  block: { hash: string; height: bigint };
  bookmark: bigint;
};

type WalletCacheBalanceInfo = {
  address: string;
  balance: { shielded: AddressBalance; unshielded: AccountBalance };
};

type WalletCacheCriteria =
  | { field: "address"; values: string[] }
  | { field: "account"; values: string[] }
  | { field: "nullifier"; values: Uint8Array[] }
  | undefined;

type WalletCacheNote = {
  address: string;
  note: Uint8Array;
  nullifier: Uint8Array;
};

type WalletCacheDbNote = Omit<WalletCacheNote, "note" | "nullifier"> & {
  note: ArrayBufferLike;
  nullifier: ArrayBufferLike;
};

type WalletCacheDbStakeInfo = {
  account: string;
  stakeInfo: Omit<StakeInfo, "amount"> & {
    amount: null | Omit<Exclude<StakeInfo["amount"], null>, "total">;
  };
};

type WalletCacheGetDataType<T extends WalletCacheTableName> =
  T extends "balancesInfo"
    ? WalletCacheBalanceInfo[]
    : T extends "pendingNotesInfo"
      ? WalletCacheDbPendingNoteInfo[]
      : T extends "stakeInfo"
        ? WalletCacheDbStakeInfo[]
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
      : T extends "stakeInfo"
        ? never
        : ArrayBuffer[];

type WalletCacheHistoryEntry = {
  history: Transaction[];
  lastBlockHeight: number;
  psk: string;
};

type WalletCacheNotesMap = Map<string, Map<Uint8Array, Uint8Array>>;

type WalletCachePendingNoteInfo = { nullifier: Uint8Array; txId: string };

type WalletCacheDbPendingNoteInfo = Omit<
  WalletCachePendingNoteInfo,
  "nullifier"
> & { nullifier: ArrayBuffer };

type WalletCacheSyncInfo = NotesSyncInfo & { lastFinalizedBlockHeight: bigint };

type WalletCacheTableName =
  | "balancesInfo"
  | "pendingNotesInfo"
  | "syncInfo"
  | "spentNotes"
  | "stakeInfo"
  | "unspentNotes";
