import { error } from "@sveltejs/kit";

import { networkStore } from "$lib/stores";

/** @type {import("./$types").PageLoad} */
export async function load() {
  return {
    currentBlockHeight: await networkStore.getCurrentBlockHeight().catch(() => {
      error(
        500,
        "Unable to retrieve current block height: the network may be down."
      );
    }),
  };
}
