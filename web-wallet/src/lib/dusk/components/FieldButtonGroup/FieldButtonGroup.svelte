<script>
  import { makeClassName } from "$lib/dusk/string";
  import { Button, Textbox } from "..";

  /** @type {TextboxTypes} */
  export let type = "text";

  /** @type {string} */
  export let name;
  export let autocomplete = "off";
  export let placeholder = "";
  export let value = "";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {string} */
  export let buttonText = "Submit";

  /** @type {Textbox} */
  let inputElement;

  export function focus() {
    inputElement?.focus();
  }

  export function select() {
    inputElement?.select();
  }

  $: classes = makeClassName(["dusk-field-button-group", className]);
</script>

<div class={classes}>
  <Textbox
    bind:this={inputElement}
    tabindex={0}
    {type}
    {autocomplete}
    {name}
    required
    bind:value
    {placeholder}
  />
  <Button on:click type="submit" text={buttonText} variant="secondary" />
</div>
