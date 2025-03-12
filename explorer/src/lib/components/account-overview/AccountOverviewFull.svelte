<script>
  import { createValueFormatter } from "$lib/dusk/value";
  import { luxToDusk } from "$lib/dusk/currency";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { OverviewPanelDetailsItem } from "$lib/components";
  import { addressCharPropertiesDefaults } from "$lib/constants";

  import "./AccountOverview.css";

  export let data;
  export let screenWidth;

  const { minScreenWidth, maxScreenWidth, minCharCount, maxCharCount } =
    addressCharPropertiesDefaults;

  const formatter = createValueFormatter("en");
</script>

<section class="account-overview-panel">
  <p class="account-overview-panel__header">
    Account:
    <b class="account-overview-panel__address">
      {middleEllipsis(
        data.address,
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
      title={`${formatter(luxToDusk(data.balance))} DUSK`}
      subtitle="Current Balance"
    />
    <OverviewPanelDetailsItem
      title={`${formatter(luxToDusk(data.staked_balance))} DUSK`}
      subtitle="Staked Balance"
    />
    <OverviewPanelDetailsItem
      title={`${formatter(luxToDusk(data.active_stake))} DUSK`}
      subtitle="Active Balance"
    />
    <OverviewPanelDetailsItem
      title={`${formatter(luxToDusk(data.inactive_stake))} DUSK`}
      subtitle="Inactive Stake"
    />
    <OverviewPanelDetailsItem
      title={`${formatter(luxToDusk(data.inactive_stake))} DUSK`}
      subtitle="Rewards"
    />
  </div>
</section>
