import { makeNodeUrl } from "$lib/url";
import { failureToRejection } from "$lib/dusk/http";

const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;
const createEmptyObject = () => ({});

/** @type {import('./$types').PageLoad} */
export async function load({ fetch }) {
  return {
    /** @type {Promise<any>} */
    transactions: fetch(
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
    )
      .then(failureToRejection)
      .then((res) => res.json())
      .catch(createEmptyObject),
  };
}
