type Readable<T> = import("svelte/store").Readable<T>;

type DataStoreContent = {
  data: any;
  error: Error | null;
  isLoading: boolean;
};

type DataStore = Readable<DataStoreContent> & {
  getData: (...args: any) => Promise<DataStoreContent>;
  reset: () => void;
};

type PollingDataStore = Readable<DataStoreContent> & {
  reset: () => void;
  start: (...args: any) => void;
  stop: (...args: any) => void;
};
