<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiKeyOutline } from "@mdi/js";
  import { validateMnemonic } from "bip39";

  import { Button, Textbox } from "$lib/dusk/components";
  import { AppAnchor, AppAnchorButton, Banner } from "$lib/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { goto } from "$lib/navigation";
  import {
    mnemonicPhraseResetStore,
    settingsStore,
    walletStore,
  } from "$lib/stores";
  import {
    decryptMnemonic,
    getSeedFromMnemonic,
    profileGeneratorFrom,
  } from "$lib/wallet";
  import loginInfoStorage from "$lib/services/loginInfoStorage";

  const localDataCheckErrorMsg =
    "Mismatched wallet address or no existing wallet";

  /** @type {(seed: Uint8Array) => Promise<import("$lib/vendor/w3sper.js/src/mod").ProfileGenerator>} */
  async function checkLocalData(seed) {
    const profileGenerator = profileGeneratorFrom(seed);
    const defaultAddress = (await profileGenerator.default).address.toString();
    const currentAddress = $settingsStore.userId;

    if (!currentAddress || currentAddress !== defaultAddress) {
      throw new Error(localDataCheckErrorMsg);
    }

    return profileGenerator;
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

  /** @type {null|"invalid-password"|"invalid-mnemonic"} */
  let error = null;

  /** @type {import("svelte/elements").FormEventHandler<HTMLFormElement>} */
  function handleUnlockWalletSubmit() {
    /** @type {(mnemonic: string) => Promise<Uint8Array>} */
    const getSeed = loginInfo
      ? getSeedFromInfo(loginInfo)
      : (mnemonic) => getSeedFromMnemonicAsync(mnemonic.toLowerCase());

    getSeed(secretText.trim())
      .then(checkLocalData)
      .then((profileGenerator) => walletStore.init(profileGenerator))
      .then(() => goto("/dashboard"))
      .catch((err) => {
        if (err.message === localDataCheckErrorMsg) {
          const enteredMnemonicPhrase = secretText.split(" ");
          mnemonicPhraseResetStore.set(enteredMnemonicPhrase);
          goto("/setup/restore");
          return;
        }
        error =
          err.message === "Wrong password"
            ? "invalid-password"
            : "invalid-mnemonic";
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
    <IconHeadingCard
      tag="article"
      gap="medium"
      heading={modeLabel}
      icons={[mdiKeyOutline]}
    >
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
        {#if error === "invalid-mnemonic"}
          <Banner title="Invalid mnemonic phrase" variant="error">
            <p>
              Please ensure you have entered your 12-word mnemonic phrase, with
              a space separating each word.
            </p>
          </Banner>
        {/if}
        {#if error === "invalid-password"}
          <Banner title="Invalid password" variant="error">
            <p>
              Please ensure the password entered matches the one you have set up
              while setting up the wallet. If you have forgotten your password,
              you can <AppAnchor href="/setup/restore">restore</AppAnchor> your wallet.
            </p>
          </Banner>
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
    </IconHeadingCard>
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
  }

  .login-footer {
    gap: var(--default-gap);
  }

  :global(.alt-login) {
    width: 100%;
  }
</style>
