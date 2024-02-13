<svelte:options immutable={true}/>

<script>
	import { createEventDispatcher, onMount } from "svelte";
	import { isType } from "lamb";
	import { Textbox } from "$lib/dusk/components";

	/** @type {Number} */
	export let limit;

	/** @type {Number} */
	export let limitLower;

	/** @type {Number} */
	export let limitUpper;

	/** @type {Number} */
	export let price;

	/** @type {Number} */
	export let priceLower;

	const dispatch = createEventDispatcher();

	function checkGasLimits () {
		let isValidPrice = false;
		let	isValidLimit = false;
		let isValidGas = false;

		if ([price, limit].every(isType("Number"))) {
			isValidPrice = price >= priceLower && price <= limit;
			isValidLimit = limit >= limitLower && limit <= limitUpper;
			isValidGas = isValidPrice && isValidLimit;
		}

		dispatch("gasSettings", {
			isValidGas: isValidGas,
			limit: limit,
			price: price
		});
	}

	onMount(() => {
		checkGasLimits();
	});
</script>

<label for={undefined} class="gas-control">
	<span class="gas-control__label">
		Price: (lux)
	</span>
	<Textbox
		bind:value={price}
		className="gas-control__input"
		max={limit}
		min={priceLower}
		on:input={checkGasLimits}
		required
		type="number"
	/>
</label>

<label for={undefined} class="gas-control">
	<span class="gas-control__label">
		Gas Limit: (unit)
	</span>
	<Textbox
		bind:value={limit}
		className="gas-control__input"
		max={limitUpper}
		min={limitLower}
		on:input={checkGasLimits}
		required
		type="number"
	/>
</label>

<style lang="postcss">
	.gas-control {
		display: flex;
		gap: 0.5em;
		width: 100%;
		flex-direction: column;
		justify-content: start;
		align-items: stretch;

		&__label {
			line-height: 140%;
		}

		:global(&__input:invalid) {
			color: var(--error-color);
		}
	}
</style>
