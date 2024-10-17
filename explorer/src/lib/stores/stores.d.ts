type AppStore = import("svelte/store").Readable<AppStoreContent> & {
  setNodeInfo: (value: NodeInfo) => void;
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
  nodeInfo: NodeInfo;
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

type NodeLocationStore =
  import("svelte/store").Readable<NodeLocationStoreContent>;

type NodeLocationStoreContent = {
  data: NodeLocation[] | null;
  error: Error | null;
  isLoading: boolean;
};

type NodeInfo = {
  bootstrapping_nodes: Array<[]>;
  chain_id: number | null | undefined;
  kadcast_address: string;
  version: string;
  version_build: string;
};
