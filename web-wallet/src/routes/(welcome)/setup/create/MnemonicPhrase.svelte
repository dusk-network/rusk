<script>
  import { Words } from "$lib/dusk/components";
  import { WarningCard } from "$lib/containers/Cards";
  import { generateMnemonic } from "$lib/wallet";

  /** @type {string[]} */
  export let mnemonicPhrase;

  if (mnemonicPhrase.length === 0) {
    mnemonicPhrase = generateMnemonic().split(" ");
  }
</script>

<WarningCard heading="Keep this SAFE">
  <p class="mnemonic-card__notice">
    Please make sure to write your phrase down and save it in a secure location.
  </p>
  <Words words={mnemonicPhrase} />
</WarningCard>

<style lang="postcss">
  .mnemonic-card__notice {
    margin-bottom: var(--small-gap);
  }
</style>
