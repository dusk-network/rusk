<svelte:options immutable={true}/>

<script>
	import { Icon, Tooltip } from "$lib/dusk/components";
	import { logo } from "$lib/dusk/icons";
	import { makeClassName } from "$lib/dusk/string";
	import { createCurrencyFormatter } from "$lib/dusk/currency";

	/** @type {String | Undefined} */
	export let className = undefined;

	/** @type {Number} */
	export let tokens = 0;

	/** @type {Number} */
	export let fiat = 0;

	/** @type {String} */
	export let tokenCurrency = "DUSK";

	/** @type {String} */
	export let fiatCurrency = "USD";

	/** @type {String} */
	export let locale = "en";

	$: classes = makeClassName([
		"dusk-balance",
		className
	]);

	const duskFormatter = createCurrencyFormatter(locale, tokenCurrency, 9);
	const fiatFormatter = createCurrencyFormatter(locale, fiatCurrency, 2);
</script>

<article
	{...$$restProps}
	class={classes}
>
	<header class="dusk-balance__header">
		<h2>Your Balance:</h2>
	</header>
	<p class="dusk-balance__dusk">
		<strong>{duskFormatter(tokens)}</strong>
		<Icon
			className="dusk-balance__icon"
			path={logo}
			data-tooltip-id="balance-tooltip"
			data-tooltip-text={tokenCurrency}
			data-tooltip-place="right"
		/>
	</p>
	<p class="dusk-balance__fiat">
		<strong>
			({fiatFormatter(fiat)})
		</strong>
	</p>
	<Tooltip id="balance-tooltip"/>
</article>
