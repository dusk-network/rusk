<script>
  import { Card, Icon, Mnemonic } from "$lib/dusk/components";
  import { arraysEqual, shuffleArray } from "$lib/dusk/array";
  import { mdiAlertOutline } from "@mdi/js";

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

<Card heading="Verification">
  <div class="flex flex-col gap-1">
    <p>Ensure you have backed up the Mnemonic phrase.</p>
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
  </div>
</Card>
