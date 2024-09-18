<script>
  import { Card, Icon } from "$lib/dusk/components";
  import { mdiAlertOutline } from "@mdi/js";
  import "./Card.css";

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
    <div
      class="dusk-card__heading-container dusk-card__heading-container--warning-card"
    >
      <Icon path={mdiAlertOutline} />
      <h3 class="h4">{heading}</h3>
    </div>
  </header>
  <slot />
</Card>
