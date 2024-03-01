<script>
  /* eslint-disable-next-line import/no-unresolved */
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import Icon from "../Icon/Icon.svelte";
  import { makeClassName } from "$lib/dusk/string";
  import { toastList, toastTimer } from "./store";
  import { onMount } from "svelte";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {Number} */
  export let timer = 2000;

  /** @type {Number} */
  export let flyDuration = 500;

  const classes = makeClassName(["dusk-toast", className]);

  onMount(() => {
    $toastTimer = timer;
  });
</script>

<ul {...$$restProps} class={classes}>
  {#each $toastList as { id, icon, message, type } (id)}
    <li
      in:fly|global={{ duration: flyDuration, x: 200 }}
      out:fly|global={{ duration: flyDuration, x: 200 }}
      animate:flip={{ duration: 200 }}
      class="dusk-toast__item"
    >
      {#if icon}
        <span
          class={`dusk-toast__item-icon-wrapper dusk-toast__item-icon-wrapper--${type}`}
        >
          <Icon className="dusk-toast__item-icon" path={icon} size="normal" />
        </span>
      {/if}
      <span class="dusk-toast__item-message">
        {message}
      </span>
    </li>
  {/each}
</ul>
