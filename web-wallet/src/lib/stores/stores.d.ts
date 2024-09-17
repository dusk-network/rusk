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

type GasSettings = {
  limit: number;
  price: number;
};

type TransactionsStoreContent = { transactions: Transaction[] };

type TransactionsStore = Readable<TransactionsStoreContent>;

type OperationsStoreContent = { currentOperation: string };

type OperationsStore = Writable<OperationsStoreContent>;

type NetworkStoreContent = {
  get connected(): boolean;
};

type NetworkStoreServices = {
  connect: () => Promise<import("$lib/vendor/w3sper.js/src/mod").Network>;
  disconnect: () => Promise<void>;
  getCurrentBlockHeight: () => Promise<bigint>;
};

type NetworkStore = Readable<NetworkStoreContent> & NetworkStoreServices;

type WalletStoreContent = {
  balance: {
    maximum: number;
    value: number;
  };
  syncStatus: {
    isInProgress: boolean;
    current: number;
    last: number;
    error: Error | null;
  };
  currentAddress: string;
  initialized: boolean;
  addresses: string[];
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

  setCurrentAddress: (address: string) => Promise<void>;

  stake: (amount: number, gasSettings: GasSettings) => Promise<any>;

  sync: (from?: bigint) => Promise<void>;

  transfer: (
    to: string,
    amount: number,
    gasSettings: GasSettings
  ) => Promise<any>;

  unstake: (gasSettings: GasSettings) => Promise<any>;

  withdrawReward: (gasSettings: GasSettings) => Promise<any>;
};

type WalletStore = Readable<WalletStoreContent> & WalletStoreServices;
