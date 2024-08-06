<svelte:options immutable={true} />

<script>
  import { isType } from "lamb";

  import { makeClassName, randomUUID } from "$lib/dusk/string";

  import "./ExclusiveChoice.css";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {string | undefined} */
  export let name = undefined;

  /** @type {SelectOption[] | String[]} */
  export let options;

  /** @type {string} */
  export let value;

  /** @type {(v: any) => v is string} */
  const isString = isType("String");

  const baseId = randomUUID();

  $: classes = makeClassName(["dusk-exclusive-choice", className]);
</script>

<div class={classes} role="radiogroup" {...$$restProps}>
  {#each options as option (option)}
    {@const isStringOption = isString(option)}
    {@const optionValue = isStringOption ? option : option.value}
    {@const id = `${baseId}-${optionValue}`}
    <input
      bind:group={value}
      class="dusk-exclusive-choice__radio"
      checked={optionValue === value}
      disabled={isStringOption ? false : option.disabled}
      {id}
      name={name ?? baseId}
      on:change
      type="radio"
      value={optionValue}
    />
    <label class="dusk-exclusive-choice__label" for={id}
      >{isStringOption ? option : option.label ?? optionValue}</label
    >
  {/each}
</div>
