<script>
  import { Icon } from "$lib/dusk/components";
  import { TokenDetailItem } from "$lib/components";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";

  import "./TokenOverview.css";

  export let data;
  export let iconPath;
  export let screenWidth;
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
            calculateAdaptiveCharCount(screenWidth, 320, 1024, 4, 30)
          )}
        </b>
      </p>
    </div>
  </div>
  <hr class="token-overview-panel__separator" />
  <div class="token-overview-panel__details">
    <TokenDetailItem
      title="{data.totalCurrentSupply} {data.ticker}"
      subtitle="Current Total Supply"
    />
    <TokenDetailItem
      title={data.maxCirculatingSupply}
      subtitle="Max Circulating Supply"
    />
    <TokenDetailItem title="${data.price}" subtitle="Price" />
  </div>
</section>
