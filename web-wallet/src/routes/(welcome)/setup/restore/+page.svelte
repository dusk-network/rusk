<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import TermsOfService from "../TermsOfService.svelte";
  import PasswordSetup from "../PasswordSetup.svelte";
  import NetworkSyncing from "./NetworkSyncSettings.svelte";
  import AllSet from "../AllSet.svelte";
  import MnemonicAuthenticate from "./MnemonicAuthenticate.svelte";
  import { Wizard, WizardStep } from "$lib/dusk/components";
  import { ExistingWalletNotice } from "$lib/components";
  import { mnemonicPhraseResetStore, settingsStore } from "$lib/stores";
  import {
    initializeWallet,
    refreshLocalStoragePasswordInfo,
  } from "$lib/wallet";
  import { goto } from "$lib/navigation";
  import { onDestroy } from "svelte";
  import NetworkSyncProgress from "./NetworkSyncProgress.svelte";

  /** @type {boolean} */
  let notice = false;

  /** @type {boolean} */
  let tosAccepted = false;

  /** @type {string} */
  let password = "";

  /** @type {boolean} */
  let isValidPassword = false;

  /** @type {boolean} */
  let isValidBlockHeight = false;

  /** @type {boolean} */
  let isSyncCompleted = false;

  /** @type {boolean} */
  let showPasswordSetup = true;

  /** @type {boolean} */
  let isValidMnemonic = false;

  /** @type {string[]} */
  let mnemonicPhrase = $mnemonicPhraseResetStore;

  /** @type {bigint} */
  let blockHeight = 0n;

  const { userId } = $settingsStore;

  $: if (showPasswordSetup) {
    password = showPasswordSetup ? password : "";
  }

  onDestroy(() => {
    mnemonicPhraseResetStore.set([]);
  });
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
  <Wizard fullHeight={true} steps={5} let:key>
    <WizardStep
      step={0}
      {key}
      showStepper={true}
      backButton={{
        disabled: false,
        href: "/setup",
        isAnchor: true,
      }}
      nextButton={{
        disabled: !isValidMnemonic,
      }}
    >
      <h2 class="h1" slot="heading">
        Enter<br />
        <mark>Mnemonic Phrase</mark>
      </h2>
      <MnemonicAuthenticate
        bind:enteredMnemonicPhrase={mnemonicPhrase}
        bind:isValid={isValidMnemonic}
      />
    </WizardStep>
    <WizardStep
      step={1}
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
      step={2}
      {key}
      showStepper={true}
      nextButton={{
        action: async () => {
          await initializeWallet(mnemonicPhrase.join(" "), blockHeight);
          mnemonicPhrase = [];
        },
        disabled: !isValidBlockHeight,
      }}
    >
      <h2 class="h1" slot="heading">
        Network<br />
        <mark>Syncing</mark>
      </h2>
      <NetworkSyncing bind:isValid={isValidBlockHeight} bind:blockHeight />
    </WizardStep>
    <WizardStep
      step={3}
      {key}
      showStepper={true}
      hideBackButton={true}
      nextButton={{ disabled: !isSyncCompleted }}
    >
      <h2 class="h1" slot="heading">
        Network<br />
        <mark>Syncing</mark>
      </h2>
      <NetworkSyncProgress bind:isValid={isSyncCompleted} />
    </WizardStep>
    <WizardStep
      step={4}
      {key}
      showStepper={true}
      hideBackButton={true}
      nextButton={{
        action: async () => {
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
