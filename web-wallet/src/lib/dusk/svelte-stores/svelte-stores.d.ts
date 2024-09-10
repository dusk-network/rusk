type DataStoreContent = {
  data: any;
  error: Error | null;
  isLoading: boolean;
};

type DataStore = Readable<DataStoreContent> & {
  getData: (...args: any) => Promise<DataStoreContent>;
  reset: () => void;
};
