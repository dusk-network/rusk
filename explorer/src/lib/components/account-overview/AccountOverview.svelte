<script>
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { addressCharPropertiesDefaults } from "$lib/constants";

  import "./AccountOverview.css";
  import { onMount } from "svelte";

  export let accountAddress;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  const { minScreenWidth, maxScreenWidth, minCharCount, maxCharCount } =
    addressCharPropertiesDefaults;
</script>

<section class="account-overview-panel">
  <p class="account-overview-panel__header">
    Account:
    <b class="account-overview-panel__address">
      {middleEllipsis(
        accountAddress,
        calculateAdaptiveCharCount(
          screenWidth,
          minScreenWidth,
          maxScreenWidth,
          minCharCount,
          maxCharCount
        )
      )}
    </b>
  </p>
</section>
