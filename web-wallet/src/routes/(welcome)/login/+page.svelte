<svelte:options immutable={true}/>

<script>
	import { mdiArrowLeft, mdiFileKeyOutline, mdiKeyOutline } from "@mdi/js";
	import { goto } from "$app/navigation";
	import { validateMnemonic } from "bip39";
	import { setKey } from "lamb";
	import {
		AnchorButton,
		Button,
		Card,
		Textbox
	} from "$lib/dusk/components";
	import { settingsStore, walletStore } from "$lib/stores";
	import { decryptMnemonic, getSeedFromMnemonic } from "$lib/wallet";
	import loginInfoStorage from "$lib/services/loginInfoStorage";
	import { getWallet } from "$lib/services/wallet";

	const notice = [
		"Logging in to a new wallet will overwrite the current local wallet cache",
		", meaning that when you log in again with the previous mnemonic/",
		"account you will need to wait for the wallet to sync."
	].join("");

	/**
	 * @typedef {import("@dusk-network/dusk-wallet-js").Wallet} Wallet
	 */

	/** @type {(wallet: Wallet) => Promise<Wallet>} */
	async function checkLocalData (wallet) {
		const defaultAddress = (await wallet.getPsks())[0];
		const currentAddress = $settingsStore.userId;

		if (defaultAddress !== currentAddress) {
			// eslint-disable-next-line no-alert
			if (currentAddress && !window.confirm(notice)) {
				throw new Error("Existing wallet detected");
			}

			await wallet.reset();
			settingsStore.reset();
			settingsStore.update(setKey("userId", defaultAddress));
		}

		return wallet;
	}

	/** @type {(mnemonic: string) => Promise<Uint8Array>} */
	const getSeedFromMnemonicAsync = async mnemonic => (
		validateMnemonic(mnemonic)
			? getSeedFromMnemonic(mnemonic)
			: Promise.reject(new Error("Invalid mnemonic"))
	);

	/** @type {(loginInfo: MnemonicEncryptInfo) => (pwd: string) => Promise<Uint8Array>} */
	const getSeedFromInfo = loginInfo => pwd => decryptMnemonic(loginInfo, pwd)
		.then(getSeedFromMnemonic, () => Promise.reject(new Error("Wrong password")));

	const loginInfo = loginInfoStorage.get();
	const modeLabel = loginInfo ? "Password" : "Mnemonic phrase";

	/** @type {Textbox} */
	let fldSecret;

	/** @type {string} */
	let secretText = "";

	/** @type {string} */
	let errorMessage = "";

	/** @type {import("svelte/elements").FormEventHandler<HTMLFormElement>} */
	function handleUnlockWalletSubmit () {
		/** @type {(mnemonic: string) => Promise<Uint8Array>} */
		const getSeed = loginInfo
			? getSeedFromInfo(loginInfo)
			: mnemonic => getSeedFromMnemonicAsync(mnemonic.toLowerCase());

		getSeed(secretText.trim())
			.then(getWallet)
			.then(checkLocalData)
			.then(wallet => walletStore.init(wallet))
			.then(() => goto("/dashboard"))
			.catch(err => {
				errorMessage = err.message;
				fldSecret.focus();
				fldSecret.select();
			});
	}
</script>

<section class="login">
	<h2 class="h1">
		Unleash <mark>RWA</mark> and<br/>
		<mark>Decentralized Finance</mark>
	</h2>
	<div class="login">
		<Card tag="article" iconPath={mdiKeyOutline} heading={modeLabel}>
			<form
				class="login__form"
				on:submit|preventDefault={handleUnlockWalletSubmit}
			>
				<Textbox
					bind:this={fldSecret}
					bind:value={secretText}
					name="secret"
					placeholder={modeLabel}
					required
					type="password"
					autocomplete={loginInfo ? "current-password" : undefined}
				/>
				{#if errorMessage}
					<span class="login__error">{errorMessage}</span>
				{/if}
				<Button variant="secondary" text="Unlock Wallet" type="submit"/>
				{#if modeLabel === "Password"}
					<AnchorButton variant="quaternary" href="/setup/restore" text="Forgot Password?"/>
				{/if}
			</form>
		</Card>
		<Card tag="article" heading="Upload DAT file" iconPath={mdiFileKeyOutline}>
			<Button
				className="alt-login"
				variant="tertiary"
				text="Choose file"
				disabled={true}/>
		</Card>
	</div>
	<footer class="login-footer">
		<AnchorButton
			href="/setup"
			variant="tertiary"
			icon={{ path: mdiArrowLeft }}
			text="Back"
		/>
	</footer>
</section>

<style lang="postcss">
	.login,
	.login-footer,
	.login__form {
		display: flex;
		flex-direction: column;
	}

	.login {
		height: 100%;
		overflow-y: auto;
		gap: var(--large-gap);

		&__form {
			gap: var(--default-gap);
		}

		&__error {
			color: var(--error);
		}
	}

	.login-footer {
		gap: var(--default-gap);
	}

	:global(.alt-login) {
		width: 100%;
	}
</style>
