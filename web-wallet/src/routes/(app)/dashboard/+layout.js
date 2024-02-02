/** @type {import('./$types').LayoutLoad} */
export async function load ({ fetch }) {
	const res = await fetch("https://api.dusk.network/v1/quote");
	const duskInfo = await res.json();

	return { currentPrice: duskInfo.market_data.current_price };
}
