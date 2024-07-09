type StorageType = "local" | "session";

type StorageSerializer = (value: any) => string;

type StorageDeserializer = (value: string) => any;

type DuskStorage = {
  clear: () => Promise<void>;
  getItem: (key: string) => Promise<any>;
  removeItem: (key: string) => Promise<void>;
  setItem: (key: string, value: any) => Promise<void>;
};
