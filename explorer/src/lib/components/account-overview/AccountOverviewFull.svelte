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
    <OverviewPanelDetailsItem subtitle="Current Balance">
      <p>{`${formatter(luxToDusk(data.balance))} DUSK`}</p>
    </OverviewPanelDetailsItem>
    <OverviewPanelDetailsItem subtitle="Staked Balance">
      <p>{`${formatter(luxToDusk(data.staked_balance))} DUSK`}</p>
    </OverviewPanelDetailsItem>
    <OverviewPanelDetailsItem subtitle="Active Balance">
      <p>{`${formatter(luxToDusk(data.active_stake))} DUSK`}</p>
    </OverviewPanelDetailsItem>
    <OverviewPanelDetailsItem subtitle="Inactive Stake">
      <p>{`${formatter(luxToDusk(data.inactive_stake))} DUSK`}</p>
    </OverviewPanelDetailsItem>
    <OverviewPanelDetailsItem subtitle="Rewards">
      <p>{`${formatter(luxToDusk(data.inactive_stake))} DUSK`}</p>
    </OverviewPanelDetailsItem>
  </div>
</section>
