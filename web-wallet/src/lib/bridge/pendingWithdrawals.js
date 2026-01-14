import { makeNodeUrl } from "$lib/url";
import { failureToRejection } from "$lib/dusk/http";

const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;

/**
 * Fetch pending withdrawals from the bridge contract.
 *
 * @param {typeof fetch} [fetchFn]
 * @returns {Promise<PendingWithdrawalEntry[]>}
 */
export async function fetchPendingWithdrawals(fetchFn = fetch) {
  try {
    const res = await fetchFn(
      makeNodeUrl(
        `/on/contracts:${VITE_BRIDGE_CONTRACT_ID}/pending_withdrawals`
      ),
      {
        body: JSON.stringify(null),
        headers: {
          "Content-Type": "application/json",
          "Rusk-feeder": "true",
        },
        method: "POST",
      }
    ).then(failureToRejection);

    const json = await res.json();
    return Array.isArray(json) ? json : [];
  } catch {
    return [];
  }
}

/**
 * Count pending withdrawals for a DuskDS account.
 *
 * @param {string} accountAddress
 * @param {PendingWithdrawalEntry[]} withdrawals
 * @returns {number}
 */
export function countPendingWithdrawalsFor(accountAddress, withdrawals) {
  if (!accountAddress || !Array.isArray(withdrawals)) {
    return 0;
  }

  return withdrawals.reduce((acc, entry) => {
    const tx = Array.isArray(entry) ? entry[1] : undefined;
    return acc + (tx?.to?.External === accountAddress ? 1 : 0);
  }, 0);
}
