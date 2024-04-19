type Readable<T> = import("svelte/store").Readable<T>;

type DataStoreContent = {
  data: any;
  error: Error | null;
  isLoading: boolean;
};

type DataStore = Readable<DataStoreContent> & {
  getData: (...args: any) => Promise<DataStoreContent>;
};

type PollingDataStore = Readable<DataStoreContent> & {
  start: (...args: any) => void;
  stop: (...args: any) => void;
};
