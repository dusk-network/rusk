<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiKeyOutline } from "@mdi/js";
  import { validateMnemonic } from "bip39";

  import { Button, Card, Textbox } from "$lib/dusk/components";
  import { AppAnchorButton } from "$lib/components";
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

  const localDataCheckErrorMsg =
    "Mismatched wallet address or no existing wallet";

  /** @type {(wallet: Wallet) => Promise<Wallet>} */
  async function checkLocalData(wallet) {
    const defaultAddress = (await wallet.getPsks())[0];
    const currentAddress = $settingsStore.userId;

    if (!currentAddress || currentAddress !== defaultAddress) {
      throw new Error(localDataCheckErrorMsg);
    }

    return wallet;
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

  /** @type {Textbox} */
  let fldSecret;

  /** @type {string} */
  let secretText = "";

  /** @type {string} */
  let errorMessage = "";

  /** @type {import("svelte/elements").FormEventHandler<HTMLFormElement>} */
  function handleUnlockWalletSubmit() {
    /** @type {(mnemonic: string) => Promise<Uint8Array>} */
    const getSeed = loginInfo
      ? getSeedFromInfo(loginInfo)
      : (mnemonic) => getSeedFromMnemonicAsync(mnemonic.toLowerCase());

    getSeed(secretText.trim())
      .then(getWallet)
      .then(checkLocalData)
      .then((wallet) => walletStore.init(wallet))
      .then(() => goto("/dashboard"))
      .catch((err) => {
        if (err.message === localDataCheckErrorMsg) {
          const enteredMnemonicPhrase = secretText.split(" ");
          mnemonicPhraseResetStore.set(enteredMnemonicPhrase);
          goto("/setup/restore");
          return;
        }
        errorMessage = err.message;
        fldSecret.focus();
        fldSecret.select();
      });
  }
</script>

<section class="login">
  <h2 class="h1">
    Unleash <mark>RWA</mark> and<br />
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
          name={loginInfo ? "password" : "mnemonic"}
          placeholder={modeLabel}
          required
          type="password"
          autocomplete="current-password"
        />
        {#if errorMessage}
          <span class="login__error">{errorMessage}</span>
        {/if}
        <Button text="Unlock Wallet" type="submit" />
        {#if modeLabel === "Password"}
          <AppAnchorButton
            variant="tertiary"
            href="/setup/restore"
            text="Forgot Password?"
          />
        {/if}
      </form>
    </Card>
  </div>
  <footer class="login-footer">
    <AppAnchorButton
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
