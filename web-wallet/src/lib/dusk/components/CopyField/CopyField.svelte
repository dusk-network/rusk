<script>
  import { Button } from "$lib/dusk/components";
  import { mdiContentCopy } from "@mdi/js";
  import { toast } from "../Toast/store";
  import Textbox from "../Textbox/Textbox.svelte";

  /** @type {string} */
  export let name;
  /** @type {string} */
  export let displayValue;
  /** @type {string} */
  export let rawValue;

  function copyToClipboard() {
    navigator.clipboard.writeText(rawValue);
    toast("success", `${name} copied`, mdiContentCopy);
  }
</script>

<div class="copy-field">
  <Textbox
    className="copy-field__content"
    value={displayValue}
    type="text"
    readOnly
  />
  <Button
    aria-label="Copy Address"
    className="copy-field__button"
    icon={{ path: mdiContentCopy }}
    on:click={copyToClipboard}
    variant="quaternary"
  />
</div>

<style lang="postcss">
  .copy-field {
    display: flex;
    align-items: center;
    gap: 1em;
  }

  :global {
    .copy-field__content {
      flex: 1;
    }
  }
</style>
