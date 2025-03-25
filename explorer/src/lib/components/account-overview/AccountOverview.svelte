<script>
  import { onMount } from "svelte";

  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { addressCharPropertiesDefaults } from "$lib/constants";
  import { OverviewPanelDetailsItem } from "$lib/components";
  import { luxToDusk } from "$lib/dusk/currency";

  import "./AccountOverview.css";
  import { createValueFormatter } from "$lib/dusk/value";

  export let accountAddress;
  export let errorFetchingAccountStatus;
  export let accountBalance;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const formatter = createValueFormatter("en");

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
  <hr class="account-overview-panel__separator" />
  <div class="account-overview-panel__details">
    <OverviewPanelDetailsItem
      title={errorFetchingAccountStatus
        ? "N/A"
        : accountBalance !== undefined
          ? `${formatter(luxToDusk(accountBalance))} DUSK`
          : "Loading..."}
      subtitle="Current Balance"
    />
  </div>
</section>
