<script>
  import { Card, Icon, Switch } from "$lib/dusk/components";
  import { createEventDispatcher } from "svelte";

  import "./Card.css";

  /** @type {string | undefined} */
  export let iconPath = undefined;

  /** @type {string} */
  export let heading;

  /** @type {CardGap} */
  export let gap = "default";

  /** @type {boolean} */
  export let onSurface = false;

  /** @type {boolean} */
  export let isToggled = false;

  const dispatch = createEventDispatcher();

  function dispatchToggleEvent() {
    dispatch("toggle", { isToggled });
  }
</script>

<Card {...$$restProps} {gap} {onSurface}>
  <header slot="header" class="dusk-card__header">
    <div class="dusk-card__header-title">
      {#if iconPath}
        <Icon path={iconPath} />
      {/if}
      <h3 class="h4">{heading}</h3>
    </div>
    <Switch onSurface bind:value={isToggled} on:change={dispatchToggleEvent} />
  </header>
  {#if isToggled}
    <slot />
  {/if}
</Card>
