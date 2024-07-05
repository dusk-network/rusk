<script>
  import { Card, Icon } from "$lib/dusk/components";

  import "./Card.css";

  /** @type {string | undefined} */
  export let iconPath = undefined;

  /** @type {string} */
  export let heading;

  /** @type {CardGap} */
  export let gap = "default";

  /** @type {boolean} */
  export let onSurface = false;
</script>

<Card {...$$restProps} {gap} {onSurface}>
  <header slot="header" class="dusk-card__header">
    <div class="dusk-card__header-title">
      {#if iconPath}
        <Icon path={iconPath} />
      {/if}
      <h3 class="h4">{heading}</h3>
    </div>
  </header>
  <slot />
</Card>
