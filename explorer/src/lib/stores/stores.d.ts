type AppStore = import("svelte/store").Readable<AppStoreContent> & {
  setNetwork: (value: string) => void;
};

type AppStoreContent = {
  fetchInterval: number;
  network: string;
  networks: NetworkOption[];
};

type NetworkOption = {
  label: string;
  value: string;
};
