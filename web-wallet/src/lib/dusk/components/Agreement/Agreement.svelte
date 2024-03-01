<script>
  import { makeClassName } from "$lib/dusk/string";
  import Checkbox from "../Checkbox/Checkbox.svelte";

  /** @type {String} */
  export let name;

  /** @type {String} */
  export let label;

  /** @type {String} */
  export let controlId = `dusk-checkbox-${Math.random().toString(36)}`;

  /** @type {Boolean} */
  export let disabled = false;

  /** @type {Boolean} */
  export let checked = false;

  /** @type {String|Undefined} */
  export let className = undefined;

  const classes = makeClassName(["dusk-agreement", className]);
</script>

<div {...$$restProps} class={classes}>
  <Checkbox
    tabindex={0}
    id={controlId}
    {name}
    {disabled}
    bind:checked
    on:change
  />
  <label for={controlId} class="dusk-agreement__label">
    {label}
  </label>
</div>
