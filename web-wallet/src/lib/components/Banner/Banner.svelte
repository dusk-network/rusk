<script>
  import { Icon } from "$lib/dusk/components";
  import {
    mdiAlertCircleOutline,
    mdiAlertDecagramOutline,
    mdiAlertOutline,
  } from "@mdi/js";

  import "./Banner.css";

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
</script>

<div class="banner banner--{variant}">
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
