<script>
	import {
		mdiAlertOutline,
		mdiCogOutline,
		mdiLink,
		mdiRestore,
		mdiTimerSand
	} from "@mdi/js";
	import {
		AnchorButton,
		Button,
		CircularIcon,
		Icon,
		ProgressBar
	} from "$lib/dusk/components";
	import { settingsStore, walletStore } from "$lib/stores";

	$: ({ network } = $settingsStore);
	$: ({ isSyncing, error } = $walletStore);

	/** @type {string} */
	let syncStatus = "";

	/** @type {string} */
	let iconPath = "";

	/** @type {string} */
	let iconVariant = "";

	$: if (isSyncing) {
		iconVariant = "warning";
		iconPath = mdiTimerSand;
		syncStatus = `Syncing with Dusk ${network}`;
	} else if (error) {
		iconVariant = "error";
		iconPath = mdiAlertOutline;
		syncStatus = `Dusk ${network} - Sync Failed`;
	} else {
		iconVariant = "success";
		iconPath = mdiLink;
		syncStatus = `Dusk ${network}`;
	}
</script>

<svelte:window on:beforeunload={event => { event.preventDefault(); }}/>

<section class="dashboard">
	<slot/>
	<footer class="footer">
		<nav class="footer__navigation">
			<div class="footer__network-status">
				<CircularIcon
					className="footer__network-status-icon footer__network-status-icon--{iconVariant}"
					color={error ? "var(--footer-icon-error-color)" : "var(--footer-icon-color)"}
					bgColor="var(--{iconVariant}-color)"
					data-tooltip-disabled={!error}
					data-tooltip-id="main-tooltip"
					data-tooltip-text={error?.message}
					data-tooltip-type="error"
				>
					<Icon
						path={iconPath}
						size="large"
					/>
				</CircularIcon>
				<div class="footer__network-message">
					<span>{syncStatus}</span>
					{#if isSyncing}
						<ProgressBar/>
					{/if}
				</div>
			</div>
			<div class="footer__actions">
				{#if error}
					<Button
						aria-label="Retry synchronization"
						className="footer__actions-button"
						data-tooltip-id="main-tooltip"
						data-tooltip-text="Retry synchronization"
						icon={{ path: mdiRestore, size: "large" }}
						on:click={() => { walletStore.sync(); }}
						variant="quaternary"
					/>
				{/if}
				<AnchorButton
					variant="quaternary"
					className="footer__anchor-button"
					icon={{ path: mdiCogOutline, size: "large" }}
					href="/settings"
					aria-label="Settings"
					data-tooltip-id="main-tooltip"
					data-tooltip-text="Settings"
				/>
			</div>
		</nav>
	</footer>
</section>

<style lang="postcss">
	.dashboard {
		position: relative;
		display: flex;
		align-content: space-between;
		gap: 1.375rem;
		flex: 1;
		flex-direction: column;
		max-height: 100%;
	}

	.footer {
		width: 100%;

		&__navigation {
			display: flex;
			justify-content: space-between;
			gap: 0.75rem;
			align-items: center;
			width: 100%;
		}

		&__actions {
			display: flex;
			flex-direction: row;
			gap: 0.75em;
			align-items: center;
		}

		&__network-status {
			display: flex;
			align-items: center;
			gap: var(--small-gap);
			line-height: 150%;
			text-transform: capitalize;
		}

		&__network-message {
			display: flex;
			flex-direction: column;
			align-items: center;
		}

		:global(.footer__network-status-icon--error) {
			cursor: help;
		}
	}
</style>
