<script>
  import { Icon } from "$lib/dusk/components";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { OverviewPanelDetailsItem } from "$lib/components";
  import { addressCharPropertiesDefaults } from "$lib/constants";

  import "./TokenOverview.css";

  export let data;
  export let iconPath;
  export let screenWidth;

  const { minScreenWidth, maxScreenWidth, minCharCount, maxCharCount } =
    addressCharPropertiesDefaults;
</script>

<section class="token-overview-panel">
  <div class="token-overview-panel__header">
    <Icon path={iconPath} size="large" />
    <div>
      <h2 class="token-overview-panel__token-name">{data.token}</h2>
      <p class="token-overview-panel__token-address-subheader">
        Address:
        <b class="token-overview-panel__token-address">
          {middleEllipsis(
            data.contractId,
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
    </div>
  </div>
  <hr class="token-overview-panel__separator" />
  <div class="token-overview-panel__details">
    <OverviewPanelDetailsItem subtitle="Current Total Supply"
      ><p>{data.totalCurrentSupply} {data.ticker}</p></OverviewPanelDetailsItem
    >
    <OverviewPanelDetailsItem subtitle="Max Circulating Supply"
      ><p>{data.maxCirculatingSupply}</p></OverviewPanelDetailsItem
    >
    <OverviewPanelDetailsItem subtitle="Price"
      ><p>${data.price}</p></OverviewPanelDetailsItem
    >
  </div>
</section>
