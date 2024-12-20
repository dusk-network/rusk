<svelte:options immutable={true} />

<script>
  import {
    mdiAlertCircleOutline,
    mdiAlertDecagramOutline,
    mdiAlertOutline,
  } from "@mdi/js";

  import { makeClassName } from "$lib/dusk/string";
  import { Icon } from "$lib/dusk/components";

  import "./Banner.css";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {string} */
  export let title;

  /** @type {BannerVariant} */
  export let variant;

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

  $: classes = makeClassName(["banner", `banner--${variant}`, className]);
</script>

<div {...$$restProps} class={classes}>
  <Icon
    path={getBannerIconPath()}
    size="large"
    className="banner__icon banner__icon--{variant}"
  />
  <div>
    <strong class="banner__title">{title}</strong>
    <slot>
      <p>No banner content provided.</p>
    </slot>
  </div>
</div>
