<script>
  import { mdiAlertOutline } from "@mdi/js";

  import { validateMnemonic } from "$lib/wallet";
  import { Mnemonic } from "$lib/dusk/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { toast } from "$lib/dusk/components/Toast/store";

  /** @type {boolean} */
  export let isValid = false;

  /** @type {number} */
  export let wordLimit = 12;

  /** @type {string[]} */
  export let enteredMnemonicPhrase = [];

  $: isValid = validateMnemonic(enteredMnemonicPhrase.join(" "));
  $: if (
    enteredMnemonicPhrase.filter((word) => word !== "").length === wordLimit &&
    !isValid
  ) {
    toast("error", "Invalid mnemonic phrase", mdiAlertOutline);
  }
</script>

<IconHeadingCard gap="medium" heading="Enter your Mnemonic Phrase">
  <Mnemonic bind:enteredMnemonicPhrase {wordLimit} type="authenticate" />
</IconHeadingCard>
