type APISearchBlockResult = {
  data: {
    data: {
      blocks: {
        header: { hash: string };
      }[];
    };
  };
};

type APISearchNoResult = {
  data: {};
};

type APISearchTransactionResult = {
  data: {
    data: {
      transactions: {
        __typename: string;
        tx: { id: string };
        txid: string;
      }[];
    };
  };
};

type APISearchResult =
  | APISearchBlockResult
  | APISearchNoResult
  | APISearchTransactionResult;
