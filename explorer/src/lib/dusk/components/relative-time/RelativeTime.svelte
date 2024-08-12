<svelte:options immutable={true} />

<script>
  import { Rerender } from "..";
  import { getRelativeTimeString, makeClassName } from "$lib/dusk/string";

  /** @type {boolean} */
  export let autoRefresh = false;

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {Date} */
  export let date;

  $: classes = makeClassName(["dusk-relative-time", className]);
</script>

<time {...$$restProps} class={classes} datetime={date.toISOString()}>
  {#if autoRefresh}
    <Rerender
      generateValue={() => getRelativeTimeString(date, "long")}
      let:value
    >
      <slot relativeTime={value}>{value}</slot>
    </Rerender>
  {:else}
    {@const relativeTime = getRelativeTimeString(date, "long")}
    <slot {relativeTime}>{relativeTime}</slot>
  {/if}
</time>
