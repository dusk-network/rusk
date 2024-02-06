<script>
	import { slide } from "svelte/transition";
	import { Button } from "$lib/dusk/components";
	import { GasControls, GasFee } from "$lib/components";
	import { createEventDispatcher, onMount } from "svelte";

	/** @type {number} */
	export let limit;

	/** @type {number} */
	export let limitLower;

	/** @type {number} */
	export let limitUpper;

	/** @type {number} */
	export let price;

	/** @type {number} */
	export let priceLower;

	/** @type {string} */
	export let fee;

	/** @type {boolean} */
	let isExpanded = false;

	const dispatch = createEventDispatcher();

	onMount(() => {
		let inputPrice = false;
		let	inputLimit = false;
		let validGasLimits = false;

		inputPrice = !!(price >= priceLower && price <= limitUpper);

		inputLimit = !!(limit >= limitLower && limit <= limitUpper);

		validGasLimits = !!(inputPrice && inputLimit);

		dispatch("checkGasLimits", validGasLimits);
	});
</script>

<div class="gas-settings">
	<dl class="gas-settings__edit">
		<GasFee {fee}/>
		<dd>
			<Button
				size="small"
				variant="tertiary"
				on:click={() => {
					isExpanded = !isExpanded;
				}}
				text={isExpanded ? "CLOSE" : "EDIT"}
			/>
		</dd>
	</dl>
	{#if isExpanded}
		<div in:slide|global class="gas-settings">
			<GasControls
				on:setGasSettings={(event) => { dispatch("setGasSettings", event.detail); }}
				on:checkGasLimits={(event) => {dispatch("checkGasLimits", event.detail); }}
				{limit}
				{limitLower}
				{limitUpper}
				{price}
				{priceLower}
			/>
		</div>
	{/if}
</div>

<style lang="postcss">
	.gas-settings {
		display: flex;
		flex-direction: column;
		gap: 0.625em;
	}

	.gas-settings__edit {
		display: flex;
		flex-direction: row;
		justify-content: space-between;
	}
</style>
