<script>
	import { slide } from "svelte/transition";
	import { Button } from "$lib/dusk/components";
	import { GasControls, GasFee } from "$lib/components";
	import { areValidGasSettings } from "$lib/contracts";
	import { onMount } from "svelte";

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

	onMount(() => {
		if (!areValidGasSettings(price, limit)) {
			isExpanded = true;
		}
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
				on:gasSettings
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
