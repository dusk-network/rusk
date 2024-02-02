<svelte:options immutable={true}/>

<script>
	import "css-doodle";
	import Animation from "./Animation.svelte";
	import { AnchorButton } from "$lib/dusk/components";
	import {
		mdiKeyOutline,
		mdiRestore,
		mdiWalletOutline
	} from "@mdi/js";
</script>

<section class="setup-content">
	<div class="setup-content__wrapper">
		<h2 class="h1 setup-heading">
			Unlocking the Future: <br/>
			Your <mark>DUSK</mark> Native Wallet
		</h2>

		<div class="setup-content__options">
			<div class="setup-content__options-group">
				<AnchorButton
					href="/setup/create"
					variant="primary"
					text="Create new wallet"
					icon={{ path: mdiWalletOutline }}/>
				<AnchorButton
					href="/setup/restore/"
					variant="tertiary"
					text="Restore Wallet"
					icon={{ path: mdiRestore }}/>
			</div>
			<Animation/>
			<AnchorButton
				href="/login"
				variant="secondary"
				text="Access wallet"
				icon={{ path: mdiKeyOutline }}/>
		</div>
	</div>
</section>

<style lang="postcss">
	.setup-content {
		flex: 1;

		&__wrapper {
			display: flex;
			flex-direction: column;
			gap: var(--large-gap);
			height: 100%;
		}

		&__options {
			display: flex;
			flex-direction: column;
			justify-content: space-between;
			height: 100%;

			&-group {
				display: flex;
				flex-direction: column;
				gap: var(--default-gap);
			}
		}
	}

	.setup-heading {
		position: relative;
	}
</style>
