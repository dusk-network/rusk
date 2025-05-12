<script>
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import { Icon } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";
  import { toastList, toastTimer } from "./store";
  import { onMount } from "svelte";

  import "./Toast.css";

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
          <Icon className="dusk-toast__item-icon" path={icon} size="default" />
        </span>
      {/if}
      <span class="dusk-toast__item-message">
        {message}
      </span>
    </li>
  {/each}
</ul>
