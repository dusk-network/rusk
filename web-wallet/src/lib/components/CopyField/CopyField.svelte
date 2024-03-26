<svelte:options immutable={true} />

<script>
  import "./CopyField.css";

  import { mdiAlertOutline, mdiContentCopy } from "@mdi/js";
  import { Button, Textbox } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";

  /** @type {string} */
  export let name;

  /** @type {string} */
  export let displayValue;

  /** @type {string} */
  export let rawValue;

  /** @type {boolean} */
  export let disabled = false;

  function copyToClipboard() {
    navigator.clipboard
      .writeText(rawValue)
      .then(() => {
        toast("success", `${name} copied`, mdiContentCopy);
      })
      .catch((err) => {
        toast(
          "error",
          err.name === "NotAllowedError"
            ? "Clipboard access denied"
            : err.message,
          mdiAlertOutline
        );
      });
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
    variant="primary"
    {disabled}
  />
</div>
