<svelte:options immutable={true} />

<script>
  import "./CopyField.css";

  import { mdiAlertOutline, mdiContentCopy } from "@mdi/js";
  import { Button, Textbox } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";
  import { makeClassName } from "$lib/dusk/string";

  /** @type {string | undefined} */
  export let className = undefined;

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

  $: classes = makeClassName(["copy-field", className]);
</script>

<div class={classes} {...$$restProps}>
  <Textbox
    className="copy-field__content"
    value={displayValue}
    type="text"
    readOnly
  />
  <Button
    aria-label="Copy Address"
    className="copy-field__button"
    data-tooltip-id="main-tooltip"
    data-tooltip-text="Copy to clipboard"
    icon={{ path: mdiContentCopy }}
    on:click={copyToClipboard}
    variant="primary"
    {disabled}
  />
</div>
