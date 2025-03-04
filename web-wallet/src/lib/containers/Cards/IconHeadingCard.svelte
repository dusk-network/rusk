<svelte:options immutable={true} />

<script>
  import { Card, Icon } from "$lib/dusk/components";
  import "./Card.css";

  /** @type {string[] | undefined} */
  export let icons = undefined;

  /** @type {string} */
  export let heading;

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {CardGap} */
  export let gap = "default";

  /** @type {boolean} */
  export let onSurface = false;
</script>

<Card {...$$restProps} {className} {gap} {onSurface}>
  <header slot="header" class="dusk-card__header">
    <div class="dusk-card__heading-container">
      <h3 class="h4">{heading}</h3>
      {#if icons && icons.length}
        <div class="dusk-card__heading-icons">
          {#each icons as iconPath (iconPath)}
            <Icon path={iconPath} />
          {/each}
        </div>
      {/if}
    </div>
  </header>
  <slot />
</Card>
