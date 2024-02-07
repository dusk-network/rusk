<script>
	import { Button, Card } from "$lib/dusk/components";
	import { AppAnchorButton } from "$lib/components";
	import { mdiAlertOutline } from "@mdi/js";

	/** @type {boolean} */
	export let notice = false;
</script>

<section class="flex flex-col flex-1">
	<Card iconPath={mdiAlertOutline} heading="Existing Wallet Detected">
		<div class="flex flex-col gap-1">
			<p>Logging in to a new wallet will overwrite the current local wallet cache,
				meaning that when you log in again with the previous mnemonic/account
				you will need to wait for the wallet to sync.
			</p>
			<div class="flex gap-1 w-100">
				<AppAnchorButton
					className="flex-1"
					href="/setup"
					text="Back"
					variant="tertiary"
				/>
				<Button
					className="flex-1"
					text="Proceed"
					on:click={() => { notice = true; }}
					variant="secondary"
				/>
			</div>
		</div>
	</Card>
</section>
