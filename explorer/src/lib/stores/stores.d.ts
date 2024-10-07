type AppStore = import("svelte/store").Readable<AppStoreContent> & {
  setTheme: (value: boolean) => void;
};

type AppStoreContent = {
  blocksListEntries: number;
  chainInfoEntries: number;
  darkMode: boolean;
  fetchInterval: number;
  isSmallScreen: boolean;
  hasTouchSupport: boolean;
  marketDataFetchInterval: number;
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
  value: URL;
};
