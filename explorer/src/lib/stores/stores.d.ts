type AppStore = import("svelte/store").Readable<AppStoreContent> & {
  setNetwork: (value: string) => void;
};

type AppStoreContent = {
  blocksListEntries: number;
  chainInfoEntries: number;
  fetchInterval: number;
  network: string;
  networks: NetworkOption[];
  transactionsListEntries: number;
};

type NetworkOption = {
  label: string;
  value: string;
};
