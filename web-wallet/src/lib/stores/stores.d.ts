type Readable<T> = import("svelte/store").Readable<T>;

type Writable<T> = import("svelte/store").Writable<T>;

type GasStoreContent = {
  gasLimitLower: number;
  gasLimitUpper: number;
  gasPriceLower: number;
};

type GasStore = Readable<GasStoreContent>;

type SettingsStoreContent = {
  currency: string;
  darkMode: boolean;
  dashboardTransactionLimit: number;
  gasLimit: number;
  gasPrice: number;
  hideStakingNotice: boolean;
  language: string;
  minAllowedStake: number;
  network: string;
  userId: string;
};

type SettingsStore = Writable<SettingsStoreContent> & { reset: () => void };

type TransactionInfo = {
  hash: string;
  nullifiers: Uint8Array[];
};

type TransactionsStoreContent = { transactions: Transaction[] };

type TransactionsStore = Readable<TransactionsStoreContent>;

type OperationsStoreContent = { currentOperation: string };

type OperationsStore = Writable<OperationsStoreContent>;

type NetworkStoreContent = {
  get connected(): boolean;
};

type NetworkSyncerOptions = {
  signal?: AbortSignal;
};

type NetworkStoreServices = {
  connect: () => Promise<import("$lib/vendor/w3sper.js/src/mod").Network>;
  disconnect: () => Promise<void>;
  getAddressSyncer: (
    options?: NetworkSyncerOptions
  ) => Promise<import("$lib/vendor/w3sper.js/src/mod").AddressSyncer>;
  getCurrentBlockHeight: () => Promise<bigint>;
};

type NetworkStore = Readable<NetworkStoreContent> & NetworkStoreServices;

type WalletStoreContent = {
  balance: {
    shielded: {
      spendable: bigint;
      value: bigint;
    };
  };
  currentProfile: import("$lib/vendor/w3sper.js/src/mod").Profile | null;
  initialized: boolean;
  profiles: Array<import("$lib/vendor/w3sper.js/src/mod").Profile>;
  syncStatus: {
    from: bigint;
    isInProgress: boolean;
    last: bigint;
    error: Error | null;
    progress: number;
  };
};

type WalletStoreServices = {
  abortSync: () => void;

  clearLocalData: () => Promise<void>;

  clearLocalDataAndInit: (
    profileGenerator: import("$lib/vendor/w3sper.js/src/mod").ProfileGenerator,
    syncFromBlock?: bigint
  ) => Promise<void>;

  getStakeInfo: () => Promise<any>;

  getTransactionsHistory: () => Promise<any>;

  init: (
    profileGenerator: import("$lib/vendor/w3sper.js/src/mod").ProfileGenerator,
    syncFromBlock?: bigint
  ) => Promise<void>;

  reset: () => void;

  setCurrentProfile: (
    profile: import("$lib/vendor/w3sper.js/src/mod").Profile
  ) => Promise<void>;

  stake: (
    amount: number,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<any>;

  sync: (fromBlock?: bigint) => Promise<void>;

  transfer: (
    to: string,
    amount: bigint,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<any>;

  unstake: (gas: import("$lib/vendor/w3sper.js/src/mod").Gas) => Promise<any>;

  withdrawReward: (
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<any>;
};

type WalletStore = Readable<WalletStoreContent> & WalletStoreServices;
