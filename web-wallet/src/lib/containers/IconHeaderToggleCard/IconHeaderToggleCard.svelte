<script>
  import { Card, Icon, Switch } from "$lib/dusk/components";

  import "./IconHeaderToggleCard.css";

  /** @type {string | undefined} */
  export let iconPath = undefined;

  /** @type {string} */
  export let heading;

  /** @type {CardGap} */
  export let gap = "default";

  /** @type {boolean} */
  export let onSurface = false;

  /** @type {boolean | undefined} */
  export let isToggled = undefined;
</script>

<Card {...$$restProps} {gap} {onSurface}>
  <header slot="header" class="card__header">
    <div class="card__header-title">
      {#if iconPath}
        <Icon path={iconPath} />
      {/if}
      <h3 class="h4">{heading}</h3>
    </div>
    {#if isToggled !== undefined}
      <Switch onSurface bind:value={isToggled} />
    {/if}
  </header>
  {#if isToggled || isToggled === undefined}
    <slot />
  {/if}
</Card>
