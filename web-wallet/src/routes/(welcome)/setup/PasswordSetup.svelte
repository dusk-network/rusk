<script>
	import {
		Card,
		Icon,
		Textbox
	} from "$lib/dusk/components";
	import { mdiAlertOutline, mdiKeyOutline } from "@mdi/js";

	/** @type {string} */
	export let password = "";

	/** @type {boolean} */
	export let isValid = false;

	/** @type {boolean} */
	export let isToggled = false;

	/** @type {string} */
	let confirmPassword = "";

	$: isValid = !isToggled
		|| ((password.length >= 8 && confirmPassword.length >= 8) && (password === confirmPassword));

	$: if (isToggled) {
		password = "";
		confirmPassword = "";
	}
</script>

<Card
	hasToggle
	bind:isToggled
	iconPath={mdiKeyOutline}
	heading="Password">
	<div class="flex flex-col gap-1">
		<p>Please store your password safely.</p>
		<Textbox
			type="password"
			autocomplete="new-password"
			bind:value={password}
			placeholder="Set Password"/>
		<div class="confirm-password">
			<Textbox
				type="password"
				autocomplete="new-password"
				bind:value={confirmPassword}
				placeholder="Confirm Password"/>
			{#if password.length < 8}
				<span class="confirm-password--meta">Password must be at least 8 characters</span>
			{:else if confirmPassword && password !== confirmPassword}
				<span
					class="confirm-password--meta
						confirm-password--meta--error">Passwords do not match</span>
			{/if}
		</div>
	</div>
</Card>

<div class="notice">
	<Icon path={mdiAlertOutline} size="large"/>
	<p>
		Setting a password for your web wallet is optional. Doing so allows you
		the convenience of opening your wallet file using a password, but it
		weakens the overall security. Not using a password requires you to input
		the full mnemonic to open your wallet.
	</p>
</div>

<style lang="postcss">
	.confirm-password {
		display: flex;
		flex-direction: column;

		&--meta {
			font-size: 0.75em;
			margin-top: 0.8em;
			margin-left: 1em;
			opacity: .5;

			&--error {
				color: var(--danger-color);
				opacity: 1;
			}
		}
	}
</style>
