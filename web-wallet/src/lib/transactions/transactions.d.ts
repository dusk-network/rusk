type Transaction = {
  amount: number;
  block_height: number;
  direction: "In" | "Out";
  fee: number;
  id: string;
  tx_type: string;
};
