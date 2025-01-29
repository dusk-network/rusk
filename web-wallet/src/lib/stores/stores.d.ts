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

type TransactionInfo = Awaited<ReturnType<Network["execute"]>>;

type TransactionsStoreContent = { transactions: Transaction[] };

type TransactionsStore = Readable<TransactionsStoreContent>;

type OperationsStoreContent = { currentOperation: string };

type OperationsStore = Writable<OperationsStoreContent>;

type NetworkStoreContent = {
  get connected(): boolean;
  networkName: string;
};

type NetworkStoreServices = {
  checkBlock: (height: bigint, hash: string) => Promise<boolean>;
  connect: () => Promise<Network>;
  disconnect: () => Promise<void>;
  getAccountSyncer: () => Promise<AccountSyncer>;
  getAddressSyncer: () => Promise<AddressSyncer>;
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
  currentProfile: Profile | null;
  initialized: boolean;
  minimumStake: bigint;
  profiles: Array<Profile>;
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

  claimRewards: (amount: bigint, gas: Gas) => Promise<TransactionInfo>;

  clearLocalData: () => Promise<void>;

  clearLocalDataAndInit: (
    profileGenerator: ProfileGenerator,
    syncFromBlock?: bigint
  ) => Promise<void>;

  getTransactionsHistory: () => Promise<any>;

  init: (
    profileGenerator: ProfileGenerator,
    syncFromBlock?: bigint
  ) => Promise<void>;

  reset: () => void;

  setCurrentProfile: (profile: Profile) => Promise<void>;

  shield: (amount: bigint, gas: Gas) => Promise<TransactionInfo>;

  stake: (amount: bigint, gas: Gas) => Promise<TransactionInfo>;

  sync: (fromBlock?: bigint) => Promise<void>;

  transfer: (
    to: string,
    amount: bigint,
    memo: string,
    gas: Gas
  ) => Promise<TransactionInfo>;

  unshield: (amount: bigint, gas: Gas) => Promise<TransactionInfo>;

  unstake: (amount: bigint, gas: Gas) => Promise<TransactionInfo>;
};

type WalletStore = Readable<WalletStoreContent> & WalletStoreServices;
