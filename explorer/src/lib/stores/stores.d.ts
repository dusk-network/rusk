type AppStore = import("svelte/store").Readable<AppStoreContent> & {
  setNetwork: (value: string) => void;
};

type AppStoreContent = {
  blocksListEntries: number;
  chainInfoEntries: number;
  fetchInterval: number;
  marketDataFetchInterval: number;
  network: string;
  networks: NetworkOption[];
  statsFetchInterval: number;
  transactionsListEntries: number;
};

type MarketDataStore =
  import("svelte/store").Readable<MarketDataStoreContent> & {
    isDataStale: () => boolean;
  };

type MarketDataStoreContent = {
  data: MarketData | null;
  error: Error | null;
  isLoading: boolean;
  lastUpdate: Date | null;
};

type NetworkOption = {
  label: string;
  value: string;
};
