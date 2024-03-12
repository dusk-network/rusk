<svelte:options immutable={true} />

<script>
  import { FieldButtonGroup } from "$lib/dusk/components";
  import { AppAnchorButton } from "$lib/components";
  import { mdiRestore, mdiWalletOutline } from "@mdi/js";

  import { validateMnemonic } from "bip39";

  import { goto } from "$lib/navigation";
  import {
    mnemonicPhraseResetStore,
    settingsStore,
    walletStore,
  } from "$lib/stores";
  import { decryptMnemonic, getSeedFromMnemonic } from "$lib/wallet";
  import loginInfoStorage from "$lib/services/loginInfoStorage";
  import { getWallet } from "$lib/services/wallet";

  /**
   * @typedef {import("@dusk-network/dusk-wallet-js").Wallet} Wallet
   */

  const existingWalletDetectedErrorMessage =
    "Mismatched wallet address detected";

  /**
   * Validates if the wallet's default address matches the current user's address.
   * @param {Wallet} wallet - The wallet object to be checked.
   */
  async function checkLocalData(wallet) {
    const [defaultAddress] = await wallet.getPsks();
    const currentUserAddress = $settingsStore.userId;

    if (defaultAddress !== currentUserAddress) {
      throw new Error(existingWalletDetectedErrorMessage);
    }
  }

  /** @type {(mnemonic: string) => Promise<Uint8Array>} */
  const getSeedFromMnemonicAsync = async (mnemonic) =>
    validateMnemonic(mnemonic)
      ? getSeedFromMnemonic(mnemonic)
      : Promise.reject(new Error("Invalid mnemonic"));

  /** @type {(loginInfo: MnemonicEncryptInfo) => (pwd: string) => Promise<Uint8Array>} */
  const getSeedFromInfo = (loginInfo) => (pwd) =>
    decryptMnemonic(loginInfo, pwd).then(getSeedFromMnemonic, () =>
      Promise.reject(new Error("Wrong password"))
    );

  const loginInfo = loginInfoStorage.get();
  const modeLabel = loginInfo ? "Password" : "Mnemonic phrase";

  /** @type {FieldButtonGroup} */
  let fldSecret;

  /** @type {string} */
  let secretText = "";

  /** @type {string} */
  let errorMessage = "";

  /** @type {import("svelte/elements").FormEventHandler<HTMLFormElement>} */
  async function handleUnlockWalletSubmit() {
    /** @type {(mnemonic: string) => Promise<Uint8Array>} */
    const getSeed = loginInfo
      ? getSeedFromInfo(loginInfo)
      : (mnemonic) => getSeedFromMnemonicAsync(mnemonic.toLowerCase());

    try {
      const seed = await getSeed(secretText.trim());
      const wallet = getWallet(seed);
      await checkLocalData(wallet);
      walletStore.init(wallet);
      goto("/dashboard");
    } catch (err) {
      if (err instanceof Error) {
        handleUnlockError(err);
      }
    }
  }

  /** @param { Error } error */
  function handleUnlockError(error) {
    if (error.message === existingWalletDetectedErrorMessage) {
      const enteredMnemonicPhrase = secretText.split(" ");
      mnemonicPhraseResetStore.set(enteredMnemonicPhrase);
      goto("/setup/restore");
      return;
    }
    errorMessage = error.message;
    fldSecret.focus();
    fldSecret.select();
  }
</script>

<section class="landing-content">
  <h2 class="h1">
    Unlocking the Future: <br />
    Your <mark>DUSK</mark> Native Wallet
  </h2>

  <div class="landing-content__options">
    <form on:submit|preventDefault={handleUnlockWalletSubmit}>
      <FieldButtonGroup
        bind:this={fldSecret}
        bind:value={secretText}
        type="password"
        name="secret"
        autocomplete="current-password"
        placeholder={modeLabel}
        buttonText="Unlock"
      />

      {#if errorMessage}
        <span class="login__error">{errorMessage}</span>
      {/if}
    </form>
    <div class="landing-content__options-group">
      <AppAnchorButton
        href="/setup/create"
        variant="primary"
        text="New"
        icon={{ path: mdiWalletOutline }}
      />
      <AppAnchorButton
        on:click={() => mnemonicPhraseResetStore.set([])}
        href="/setup/restore"
        variant="tertiary"
        text="Restore"
        icon={{ path: mdiRestore }}
      />
    </div>
  </div>
</section>

<footer class="landing-footer">
  <span
    >Web Wallet v{import.meta.env.APP_VERSION} ({import.meta.env
      .APP_BUILD_INFO})</span
  >
</footer>

<style lang="postcss">
  .landing-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--large-gap);
    height: 100%;

    &__options {
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      height: 100%;

      &-group {
        display: grid;
        gap: var(--default-gap);
        width: 100%;
        grid-template-columns: 1fr 1fr;
      }
    }
  }

  .landing-footer {
    font-size: 0.75em;
  }

  .login__error {
    color: var(--error);
    display: block;
    margin: 0.5em 1em;
  }
</style>
