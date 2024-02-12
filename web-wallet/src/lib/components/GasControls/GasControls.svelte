<svelte:options immutable={true}/>

<script>
	import { createEventDispatcher, onMount } from "svelte";
	import {
		always,
		clamp,
		clampWithin,
		compose,
		isType,
		when
	} from "lamb";

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

	// browsers may allow input of invalid characters
	const toNumber = compose(when(isNaN, always(0)), n => parseInt(n, 10));
	const toValidLimit = compose(clampWithin(limitLower, limitUpper), toNumber);

	/**
	 * @param {Number} n
	 * @param {Number} upperLimit
	 * @returns {Number}
	 */
	const toValidPrice = (n, upperLimit) => clamp(toNumber(n), priceLower, upperLimit);

	function dispatchGasChange () {
		const validLimit = toValidLimit(limit);

		dispatch("setGasSettings", {
			limit: validLimit,
			price: toValidPrice(price, validLimit)
		});

		checkGasLimits();
	}

	function handleLimitChange () {
		const newLimit = toValidLimit(limit);

		if (price > newLimit) {
			price = toValidPrice(price, newLimit);
		}

		dispatchGasChange();
		checkGasLimits();
	}

	function checkGasLimits () {
		let isValidPrice = false;
		let	isValidLimit = false;
		let isValidGas = false;

		if ([price, limit].every(isType("Number"))) {
			isValidPrice = price >= priceLower && price <= limit;
			isValidLimit = limit >= limitLower && limit <= limitUpper;
			isValidGas = isValidPrice && isValidLimit;
		}

		dispatch("gasSettingsValidity", isValidGas);
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
		max={toValidLimit(limit)}
		min={priceLower}
		on:blur={() => {
			price = toValidPrice(price, limit);
			checkGasLimits();
		}}
		on:input={dispatchGasChange}
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
		on:blur={() => {
			limit = toValidLimit(limit);
			checkGasLimits();
		}}
		on:input={handleLimitChange}
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
