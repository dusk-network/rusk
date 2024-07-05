<script>
  import { Icon, Mnemonic } from "$lib/dusk/components";
  import { arraysEqual, shuffleArray } from "$lib/dusk/array";
  import { mdiAlertOutline } from "@mdi/js";
  import { IconHeadingCard } from "$lib/containers/Cards";

  /** @type {boolean} */
  export let isValid = false;

  /** @type {string[]} */
  export let enteredMnemonicPhrase = [];

  /** @type {string[]} */
  export let mnemonicPhrase = [];

  $: filteredMnemonic = new Set(
    enteredMnemonicPhrase.filter((word) => word !== "")
  );
  $: isValid = arraysEqual(enteredMnemonicPhrase, mnemonicPhrase);
</script>

<IconHeadingCard heading="Verification">
  <p class="mnemonic-notice">Ensure you have backed up the Mnemonic phrase.</p>
  <Mnemonic
    bind:enteredMnemonicPhrase
    mnemonicPhrase={shuffleArray(mnemonicPhrase)}
    type="validate"
  />
  {#if filteredMnemonic.size === 12 && !isValid}
    <div class="notice notice--error">
      <Icon path={mdiAlertOutline} size="large" />
      <p>Mnemonic does not match.</p>
    </div>
  {/if}
</IconHeadingCard>

<style>
  .mnemonic-notice {
    margin-bottom: var(--small-gap);
  }
</style>
