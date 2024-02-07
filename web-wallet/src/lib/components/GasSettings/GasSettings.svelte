<script>
	import { Button } from "$lib/dusk/components";
	import { GasControls, GasFee } from "$lib/components";
	import { createEventDispatcher } from "svelte";

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
	<div class="gas-settings" style="display: { isExpanded ? "block" : "none" }">
		<GasControls
			on:setGasSettings={(event) => { dispatch("setGasSettings", event.detail); }}
			on:checkGasLimits={(event) => { dispatch("checkGasLimits", event.detail); }}
			{limit}
			{limitLower}
			{limitUpper}
			{price}
			{priceLower}
		/>
	</div>
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
