<script>
  import { Words } from "$lib/dusk/components";
  import { IconHeaderToggleCard } from "$lib/containers";

  import { mdiAlertOutline } from "@mdi/js";
  import { generateMnemonic } from "bip39";

  /** @type {string[]} */
  export let mnemonicPhrase;

  if (mnemonicPhrase.length === 0) {
    mnemonicPhrase = generateMnemonic().split(" ");
  }
</script>

<IconHeaderToggleCard heading="Keep this SAFE" iconPath={mdiAlertOutline}>
  <p class="mnemonic-card__notice">
    Please make sure to write your phrase down and save it in a secure location.
  </p>
  <Words words={mnemonicPhrase} />
</IconHeaderToggleCard>

<style lang="postcss">
  .mnemonic-card__notice {
    margin-bottom: var(--small-gap);
  }
</style>
