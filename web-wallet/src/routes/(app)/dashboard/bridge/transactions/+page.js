import { isUndefined, when } from "lamb";

import { makeNodeUrl } from "$lib/url";
import { failureToRejection } from "$lib/dusk/http";

const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;
const createEmptyObject = () => ({});

// curl -L -X POST 'https://devnet.nodes.dusk.network/on/contracts:[contract_id]/[fn_name]' --header 'Content-Type: application/json' --header 'Rusk-feeder: true' --data-raw 'null'

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
      //   .then(getPath("market_data.current_price"))
      .then(when(isUndefined, createEmptyObject))
      .catch(createEmptyObject),
  };
}
