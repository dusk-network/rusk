import { fetchPendingWithdrawals } from "$lib/bridge/pendingWithdrawals";

/** @type {import('./$types').LayoutLoad} */
export async function load({ fetch }) {
  return {
    /** @type {Promise<PendingWithdrawalEntry[]>} */
    pendingWithdrawals: fetchPendingWithdrawals(fetch),
  };
}
