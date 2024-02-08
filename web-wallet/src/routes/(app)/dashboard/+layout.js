import { getPath, isUndefined, when } from "lamb";

import { failureToRejection } from "$lib/dusk/http";

const createEmptyObject = () => ({});

/** @type {import('./$types').LayoutLoad} */
export async function load ({ fetch }) {
	return {
		currentPrice: await fetch("https://api.dusk.network/v1/quote")
			.then(failureToRejection)
			.then(res => res.json())
			.then(getPath("market_data.current_price"))
			.then(when(isUndefined, createEmptyObject))
			.catch(createEmptyObject)
	};
}
