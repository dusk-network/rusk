type PendingWithdrawalAddress = {
  /** DuskDS account address */
  External?: string;
  /** The contract may use a tagged enum for the destination. */
  [key: string]: unknown;
};

type PendingWithdrawalTx = {
  /** Amount in Lux (1e9) as returned by the contract. */
  amount: string | number | bigint;
  /** Block height at which the withdrawal request was created. */
  block_height: number;
  /** Source EVM address. */
  from: string;
  /** Destination on DuskDS (most commonly { External: <address> }). */
  to: PendingWithdrawalAddress;
};

/**
 * Tuple returned from `pending_withdrawals`:
 * [withdrawalId, tx]
 */
type PendingWithdrawalEntry = [bigint, PendingWithdrawalTx];
