<svelte:options immutable={true}/>

<script>
	import { Icon } from "$lib/dusk/components";
	import { logo } from "$lib/dusk/icons";

	/** @type {ContractStatus[]} */
	export let items;
</script>

<dl class="contract-statuses">
	{#each items as status (status.label)}
		<dt class="contract-statuses__label">
			{status.label}
		</dt>
		<dd class="contract-statuses__value">
			<span>{status.value}</span>
			<Icon
				className="contract-statuses__icon"
				path={logo}
				data-tooltip-id="main-tooltip"
				data-tooltip-text="DUSK"
			/>
		</dd>
	{/each}
</dl>

<style lang="postcss">
	.contract-statuses {
		background-color: var(--background-color);
		border-radius: var(--control-border-radius-size);
		padding: 0.5em 1em;
		width: 100%;
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		gap: var(--small-gap);

		&__label, &__value {
			flex: 1 1 calc(50% - var(--small-gap) / 2);
		}

		&__value {
			display: inline-flex;
			align-items: center;
			gap: var(--small-gap);
			text-align: right;
			font-weight: 600;
			justify-content: end;
		}

		:global(&__icon) {
			width: 1em;
			height: 1em;
			flex-shrink: 0;
		}
	}
</style>
