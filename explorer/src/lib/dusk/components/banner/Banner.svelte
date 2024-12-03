<script>
  import { makeClassName } from "$lib/dusk/string";
  import { Icon } from "$lib/dusk/components";
  import {
    mdiAlertCircleOutline,
    mdiAlertDecagramOutline,
    mdiAlertOutline,
  } from "@mdi/js";

  import "./Banner.css";

  /** @type {string} */
  export let title;

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {BannerVariant} */
  export let variant = "info";

  function getBannerIconPath() {
    switch (variant) {
      case "warning":
        return mdiAlertOutline;
      case "error":
        return mdiAlertDecagramOutline;
      default:
        return mdiAlertCircleOutline;
    }
  }

  $: classes = makeClassName([
    "dusk-banner",
    `dusk-banner--${variant}`,
    className,
  ]);
</script>

<div {...$$restProps} class={classes}>
  <Icon
    path={getBannerIconPath()}
    size="large"
    className="dusk-banner__icon dusk-banner__icon--{variant}"
  />
  <div>
    <strong class="dusk-banner__title">{title}</strong>
    <slot>
      <p>No banner content provided.</p>
    </slot>
  </div>
</div>
