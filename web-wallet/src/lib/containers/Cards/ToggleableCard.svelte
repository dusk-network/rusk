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

<Card {...$$restProps} {gap} {onSurface} showBody={isToggled}>
  <header slot="header" class="dusk-card__header dusk-card__header-toggle">
    <h3 class="h4">{heading}</h3>
    <div class="dusk-card__header-controls-wrapper">
      <Switch
        onSurface
        bind:value={isToggled}
        on:change={dispatchToggleEvent}
      />
      {#if iconPath}
        <Icon path={iconPath} />
      {/if}
    </div>
  </header>
  {#if isToggled}
    <slot />
  {/if}
</Card>
