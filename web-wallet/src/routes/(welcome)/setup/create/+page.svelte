<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import {
    initializeWallet,
    refreshLocalStoragePasswordInfo,
  } from "$lib/wallet";
  import { Wizard, WizardStep } from "$lib/dusk/components";
  import { ExistingWalletNotice } from "$lib/components";
  import loginInfoStorage from "$lib/services/loginInfoStorage";
  import { settingsStore } from "$lib/stores";
  import TermsOfService from "../TermsOfService.svelte";
  import MnemonicPhrase from "./MnemonicPhrase.svelte";
  import MnemonicValidate from "./MnemonicValidate.svelte";
  import SwapNDUSK from "./SwapNDUSK.svelte";
  import AllSet from "../AllSet.svelte";
  import MnemonicPreSetup from "./MnemonicPreSetup.svelte";
  import PasswordSetup from "../PasswordSetup.svelte";
  import { goto } from "$lib/navigation";

  /** @type {boolean} */
  let notice = false;

  /** @type {boolean} */
  let tosAccepted = false;

  /** @type {string} */
  let password = "";

  /** @type {boolean} */
  let isValidPassword = false;

  /** @type {boolean} */
  let showPasswordSetup = false;

  /** @type {boolean} */
  let agreementAccepted = false;

  /** @type {boolean} */
  let isValidMnemonic = false;

  /** @type {string[]} */
  let mnemonicPhrase = [];

  /** @type {string[]} */
  let enteredMnemonicPhrase = [];

  const { userId } = $settingsStore;

  $: if (showPasswordSetup) {
    password = showPasswordSetup ? password : "";
  }
</script>

{#if !notice && userId}
  <div class="onboarding-wrapper" in:fade|global>
    <ExistingWalletNotice bind:notice />
  </div>
{:else if !tosAccepted}
  <div class="onboarding-wrapper" in:fade|global>
    <TermsOfService bind:tosAccepted />
  </div>
{:else}
  <Wizard fullHeight={true} steps={6} let:key>
    <WizardStep
      step={0}
      {key}
      showStepper={true}
      backButton={{
        disabled: false,
        href: "/",
        isAnchor: true,
      }}
      nextButton={{ disabled: !agreementAccepted }}
    >
      <h2 class="h1" slot="heading">
        Backup<br />
        <mark>Mnemonic Phrase</mark>
      </h2>
      <MnemonicPreSetup bind:isValid={agreementAccepted} />
    </WizardStep>
    <WizardStep step={1} {key} showStepper={true}>
      <h2 class="h1" slot="heading">
        Backup<br />
        <mark>Mnemonic Phrase</mark>
      </h2>
      <MnemonicPhrase bind:mnemonicPhrase />
    </WizardStep>
    <WizardStep
      step={2}
      {key}
      showStepper={true}
      backButton={{
        action: () => {
          enteredMnemonicPhrase = [];
        },
      }}
      nextButton={{
        disabled: !isValidMnemonic,
      }}
    >
      <h2 class="h1" slot="heading">
        Backup<br />
        <mark>Mnemonic Phrase</mark>
      </h2>
      <MnemonicValidate
        bind:isValid={isValidMnemonic}
        bind:enteredMnemonicPhrase
        bind:mnemonicPhrase
      />
    </WizardStep>
    <WizardStep
      step={3}
      {key}
      showStepper={true}
      nextButton={{
        action: async () => {
          await refreshLocalStoragePasswordInfo(mnemonicPhrase, password);
        },
        disabled: !isValidPassword,
      }}
    >
      <h2 class="h1" slot="heading">
        <mark>Password</mark><br />
        Setup
      </h2>
      <PasswordSetup
        bind:password
        bind:isValid={isValidPassword}
        bind:isToggled={showPasswordSetup}
      />
    </WizardStep>
    <WizardStep
      step={4}
      {key}
      showStepper={true}
      backButton={{
        action: () => loginInfoStorage.remove(),
        disabled: false,
      }}
    >
      <h2 class="h1" slot="heading">
        Swap ERC20<br />
        to <mark>Native Dusk</mark>
      </h2>
      <SwapNDUSK />
    </WizardStep>
    <WizardStep
      step={5}
      {key}
      showStepper={true}
      hideBackButton={true}
      nextButton={{
        action: async () => {
          await initializeWallet(mnemonicPhrase);
          mnemonicPhrase = [];
          await goto("/dashboard");
        },
        disabled: false,
      }}
    >
      <h2 class="h1" slot="heading">
        Welcome to<br />
        <mark>Dusk</mark>
      </h2>
      <AllSet />
    </WizardStep>
  </Wizard>
{/if}

<style>
  .onboarding-wrapper {
    height: 100%;
    overflow-y: auto;
  }
</style>
