<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiKeyOutline } from "@mdi/js";
  import { validateMnemonic } from "bip39";

  import { getErrorFrom } from "$lib/dusk/error";
  import { Button, Textbox } from "$lib/dusk/components";
  import {
    InvalidMnemonicError,
    InvalidPasswordError,
    MismatchedWalletError,
  } from "$lib/errors";

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

  /** @type {(seed: Uint8Array) => Promise<import("$lib/vendor/w3sper.js/src/mod").ProfileGenerator>} */
  async function checkLocalData(seed) {
    const profileGenerator = await profileGeneratorFrom(seed);
    const defaultAddress = (await profileGenerator.default).address.toString();
    const currentAddress = $settingsStore.userId;

    if (!currentAddress || currentAddress !== defaultAddress) {
      throw new MismatchedWalletError();
    }

    return profileGenerator;
  }

  /** @type {(mnemonic: string) => Promise<Uint8Array>} */
  const getSeedFromMnemonicAsync = async (mnemonic) =>
    validateMnemonic(mnemonic)
      ? getSeedFromMnemonic(mnemonic)
      : Promise.reject(new InvalidMnemonicError());

  /** @type {(loginInfo: WalletEncryptInfo) => (pwd: string) => Promise<Uint8Array>} */
  const getSeedFromInfo = (loginInfo) => (pwd) =>
    decryptMnemonic(loginInfo, pwd).then(getSeedFromMnemonic, () =>
      Promise.reject(new InvalidPasswordError())
    );

  const loginInfo = loginInfoStorage.get();
  const modeLabel = loginInfo ? "Password" : "Mnemonic phrase";

  /** @type {Textbox} */
  let fldSecret;

  /** @type {string} */
  let secretText = "";

  /** @type {Error} */
  let error;

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
        if (err instanceof MismatchedWalletError) {
          const enteredMnemonicPhrase = secretText.split(" ");
          mnemonicPhraseResetStore.set(enteredMnemonicPhrase);
          goto("/setup/restore");

          return;
        } else {
          error = err instanceof Error ? err : getErrorFrom(err);
        }

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
          name={modeLabel}
          aria-label={modeLabel}
          placeholder={modeLabel}
          required
          type="password"
          autocomplete="current-password"
          autofocus
        />
        {#if error instanceof InvalidMnemonicError}
          <Banner title="Invalid mnemonic phrase" variant="error">
            <p>
              Please ensure you have entered your 12-word mnemonic phrase, with
              a space separating each word.
            </p>
          </Banner>
        {:else if error instanceof InvalidPasswordError}
          <Banner title="Invalid password" variant="error">
            <p>
              Please ensure the password entered matches the one you have set up
              while setting up the wallet. If you have forgotten your password,
              you can <AppAnchor href="/setup/restore">restore</AppAnchor> your wallet.
            </p>
          </Banner>
        {:else if error}
          <Banner
            title={error.name.replace(/(\w)([A-Z])/g, "$1 $2")}
            variant="error"
          >
            <p>{error.message}</p>
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
