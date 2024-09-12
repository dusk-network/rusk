import { getPath, isUndefined, when } from "lamb";

import { failureToRejection } from "$lib/dusk/http";

const createEmptyObject = () => ({});

/* eslint-disable camelcase */

const coinGeckoParams = new URLSearchParams({
  community_data: "false",
  developer_data: "false",
  localization: "false",
  market_data: "true",
  sparkline: "false",
  tickers: "false",
});

/* eslint-enable camelcase */

/** @type {import('./$types').LayoutLoad} */
export async function load({ fetch }) {
  return {
    /** @type {Promise<Record<string, number>>} */
    currentPrice: fetch(
      `https://api.coingecko.com/api/v3/coins/dusk-network?${coinGeckoParams}`
    )
      .then(failureToRejection)
      .then((res) => res.json())
      .then(getPath("market_data.current_price"))
      .then(when(isUndefined, createEmptyObject))
      .catch(createEmptyObject),
  };
}
