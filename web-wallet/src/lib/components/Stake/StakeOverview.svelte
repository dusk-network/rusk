<svelte:options immutable={true}/>

<script>
	import { mdiArrowDownBoldBoxOutline } from "@mdi/js";

	import { logo } from "$lib/dusk/icons";
	import { Icon } from "$lib/dusk/components";

	/** @type {string} */
	export let label;

	/** @type {string} */
	export let value;
</script>

<dl class="stake-overview">
	<dt class="stake-overview__label">
		<Icon path={mdiArrowDownBoldBoxOutline}/>
		<span>{label}:</span>
	</dt>
	<dd class="stake-overview__value">
		<span>{value}</span>
		<Icon
			className="stake-overview__icon"
			data-tooltip-id="main-tooltip"
			data-tooltip-text="DUSK"
			path={logo}
		/>
	</dd>
</dl>

<style lang="postcss">
	.stake-overview {
		display: flex;
		flex-direction: column;
		gap: 0.625em;

		&__value {
			display: inline-flex;
			align-items: center;
			gap: var(--small-gap);

			& > span {
				font-weight: 600;
			}
		}

		:global(&__icon) {
			width: 1em;
			height: 1em;
		}
	}
</style>
