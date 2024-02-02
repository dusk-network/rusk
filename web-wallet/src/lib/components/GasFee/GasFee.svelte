<script>
	import { logo } from "$lib/dusk/icons";
	import { Icon } from "$lib/dusk/components";

	/** @type {string} */
	export let fee;
</script>

<div class="gas-fee">
	<span>Max Gas Fee:</span>
	<div class="gas-fee__amount">
		<span class="gas-fee__amount-value">{fee}</span>
		<Icon
			className="dusk-amount__icon"
			path={logo}
			data-tooltip-id="main-tooltip"
			data-tooltip-text="DUSK"
		/>
	</div>

</div>

<style lang="postcss">
    .gas-fee {
        display: inline-flex;
        justify-content: flex-start;
		align-items: center;
		gap: var(--small-gap);
		flex-wrap: wrap;

		&__amount {
			display: inline-flex;
			gap: var(--small-gap);
			align-items: center;
		}

        &__amount-value {
            font-weight: 600;
        }
    }

    :global(.dusk-amount__icon) {
        width: 1em;
        height: 1em;
        flex-shrink: 0;
    }
</style>
