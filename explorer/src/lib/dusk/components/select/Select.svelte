<svelte:options immutable={true} />

<script>
  import { ownPairs } from "lamb";

  import { makeClassName } from "$lib/dusk/string";

  import Options from "./Options.svelte";

  import "./Select.css";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {GroupedSelectOptions | SelectOption[] | String[]} */
  export let options;

  /** @type {String | Undefined} */
  export let value = undefined;
</script>

<select
  {...$$restProps}
  bind:value
  class={makeClassName(["dusk-select", className])}
  on:change
>
  {#if Array.isArray(options)}
    <Options {options} />
  {:else}
    {#each ownPairs(options) as [label, opts] (label)}
      <optgroup {label}>
        <Options options={opts} />
      </optgroup>
    {/each}
  {/if}
</select>
