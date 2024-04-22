<script>
  import { Card, Mnemonic } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";
  import { validateMnemonic, wordlists } from "bip39";
  import { mdiAlertOutline } from "@mdi/js";

  /** @type {boolean} */
  export let isValid = false;

  /** @type {number} */
  export let wordLimit = 12;

  /** @type {string[]} */
  export let enteredMnemonicPhrase = [];

  $: isValid = validateMnemonic(
    enteredMnemonicPhrase.join(" "),
    wordlists.english
  );
  $: if (
    enteredMnemonicPhrase.filter((word) => word !== "").length === wordLimit &&
    !isValid
  ) {
    toast("error", "Invalid mnemonic phrase", mdiAlertOutline);
  }
</script>

<Card heading="Enter your Mnemonic Phrase">
  <Mnemonic bind:enteredMnemonicPhrase {wordLimit} type="authenticate" />
</Card>
