type Transaction = {
  amount: number;
  block_height: number;
  direction: "In" | "Out";
  fee: number;
  id: string;
  memo: string | Uint8Array;
  tx_type: string;
};
