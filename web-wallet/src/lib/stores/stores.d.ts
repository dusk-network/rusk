type Readable<T> = import("svelte/store").Readable<T>;

type Writable<T> = import("svelte/store").Writable<T>;

type GasStoreContent = {
  gasLimitLower: bigint;
  gasLimitUpper: bigint;
  gasPriceLower: bigint;
};

type GasStore = Readable<GasStoreContent>;

type SettingsStoreContent = {
  currency: string;
  darkMode: boolean;
  dashboardTransactionLimit: number;
  gasLimit: bigint;
  gasPrice: bigint;
  hideStakingNotice: boolean;
  language: string;
  userId: string;
  walletCreationBlockHeight: bigint;
};

type SettingsStore = Writable<SettingsStoreContent> & {
  reset: () => void;
  resetGasSettings: () => void;
};

type TransactionInfo =
  | {
      hash: string;
      nullifiers: Uint8Array[];
    }
  | {
      hash: string;
      nonce: bigint;
    };

type TransactionsStoreContent = { transactions: Transaction[] };

type TransactionsStore = Readable<TransactionsStoreContent>;

type OperationsStoreContent = { currentOperation: string };

type OperationsStore = Writable<OperationsStoreContent>;

type NetworkStoreContent = {
  get connected(): boolean;
  networkName: string;
};

type NetworkSyncerOptions = {
  signal?: AbortSignal;
};

type NetworkStoreServices = {
  checkBlock: (height: bigint, hash: string) => Promise<boolean>;
  connect: () => Promise<import("$lib/vendor/w3sper.js/src/mod").Network>;
  disconnect: () => Promise<void>;
  getAccountSyncer: (
    options?: NetworkSyncerOptions
  ) => Promise<import("$lib/vendor/w3sper.js/src/mod").AccountSyncer>;
  getAddressSyncer: (
    options?: NetworkSyncerOptions
  ) => Promise<import("$lib/vendor/w3sper.js/src/mod").AddressSyncer>;
  getBlockHashByHeight: (height: bigint) => Promise<string>;
  getCurrentBlockHeight: () => Promise<bigint>;
  getLastFinalizedBlockHeight: () => Promise<bigint>;
  init: () => Promise<void>;
};

type NetworkStore = Readable<NetworkStoreContent> & NetworkStoreServices;

type NodeInfo = {
  bootstrappingNodes: Array<string>;
  chainId: number;
  kadcastAddress: string;
  version: string;
  versionBuild: string;
};

type WalletStoreBalance = {
  shielded: AddressBalance;
  unshielded: AccountBalance;
};

type WalletStoreContent = {
  balance: WalletStoreBalance;
  currentProfile: import("$lib/vendor/w3sper.js/src/mod").Profile | null;
  initialized: boolean;
  minimumStake: bigint;
  profiles: Array<import("$lib/vendor/w3sper.js/src/mod").Profile>;
  stakeInfo: StakeInfo;
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

  getTransactionsHistory: () => Promise<any>;

  init: (
    profileGenerator: import("$lib/vendor/w3sper.js/src/mod").ProfileGenerator,
    syncFromBlock?: bigint
  ) => Promise<void>;

  reset: () => void;

  setCurrentProfile: (
    profile: import("$lib/vendor/w3sper.js/src/mod").Profile
  ) => Promise<void>;

  shield: (
    amount: bigint,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<TransactionInfo>;

  stake: (
    amount: bigint,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<TransactionInfo>;

  sync: (fromBlock?: bigint) => Promise<void>;

  transfer: (
    to: string,
    amount: bigint,
    memo: string,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<TransactionInfo>;

  unshield: (
    amount: bigint,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<TransactionInfo>;

  unstake: (
    amount: bigint,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<TransactionInfo>;

  claimRewards: (
    amount: bigint,
    gas: import("$lib/vendor/w3sper.js/src/mod").Gas
  ) => Promise<TransactionInfo>;
};

type WalletStore = Readable<WalletStoreContent> & WalletStoreServices;
