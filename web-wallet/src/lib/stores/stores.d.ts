type Readable<T> = import("svelte/store").Readable<T>;

type Wallet = import("@dusk-network/dusk-wallet-js").Wallet;

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

type TransactionsStoreContent = { transactions: Transaction[] };

type TransactionsStore = Readable<TransactionsStoreContent>;

type WalletStoreContent = {
  balance: {
    maximum: number;
    value: number;
  };
  currentAddress: string;
  error: Error | null;
  initialized: boolean;
  addresses: string[];
  isSyncing: boolean;
};

type WalletStoreServices = {
  abortSync: () => void;

  clearLocalData: () => Promise<void>;

  clearLocalDataAndInit: (wallet: Wallet) => Promise<void>;

  getStakeInfo: () => Promise<any> & ReturnType<Wallet["stakeInfo"]>;

  // The return type apparently is not in a promise here
  getTransactionsHistory: () => Promise<ReturnType<Wallet["history"]>>;

  init: (wallet: Wallet) => Promise<void>;

  reset: () => void;

  setCurrentAddress: (address: string) => Promise<void>;

  stake: (
    amount: number,
    gasPrice: number,
    gasLimit: number
  ) => Promise<any> & ReturnType<Wallet["stake"]>;

  sync: () => Promise<void>;

  transfer: (
    to: string,
    amount: number,
    gasPrice: number,
    gasLimit: number
  ) => Promise<any> & ReturnType<Wallet["transfer"]>;

  unstake: (
    gasPrice: number,
    gasLimit: number
  ) => Promise<any> & ReturnType<Wallet["unstake"]>;

  withdrawReward: (
    gasPrice: number,
    gasLimit: number
  ) => Promise<any> & ReturnType<Wallet["withdrawReward"]>;
};

type WalletStore = Readable<WalletStoreContent> & WalletStoreServices;
