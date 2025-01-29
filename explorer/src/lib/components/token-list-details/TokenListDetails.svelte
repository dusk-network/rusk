<svelte:options immutable={true} />

<script>
  import { AppAnchor, DetailList, ListItem } from "$lib/components";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { onMount } from "svelte";

  /** @type {Token} */
  export let data;

  /** @type {Boolean} */
  export let displayTooltips = false;

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

  $: adaptiveCharCount = calculateAdaptiveCharCount(
    screenWidth,
    320,
    1024,
    4,
    25
  );
  $: tokensContractID = middleEllipsis(data.contractId, adaptiveCharCount);
</script>

<DetailList>
  <!-- TOKEN NAME -->
  <ListItem tooltipText={displayTooltips ? "The name of the token." : ""}>
    <svelte:fragment slot="term">Token</svelte:fragment>
    <svelte:fragment slot="definition">
      <AppAnchor href={`/tokens/token/${data.token}`}>{data.token}</AppAnchor>
    </svelte:fragment>
  </ListItem>

  <!-- TOTAL CURRENT SUPPLY -->
  <ListItem
    tooltipText={displayTooltips
      ? "The total amount of tokens currently in circulation."
      : ""}
  >
    <svelte:fragment slot="term">Total Current Supply</svelte:fragment>
    <svelte:fragment slot="definition">
      {data.totalCurrentSupply}
    </svelte:fragment>
  </ListItem>

  <!-- MAX CIRCULATING SUPPLY -->
  <ListItem
    tooltipText={displayTooltips
      ? "The maximum number of tokens that can ever exist."
      : ""}
  >
    <svelte:fragment slot="term">Max Circulating Supply</svelte:fragment>
    <svelte:fragment slot="definition">
      {data.maxCirculatingSupply}
    </svelte:fragment>
  </ListItem>

  <!-- TICKER SYMBOL -->
  <ListItem
    tooltipText={displayTooltips
      ? "The ticker symbol used to identify the token."
      : ""}
  >
    <svelte:fragment slot="term">Ticker</svelte:fragment>
    <svelte:fragment slot="definition">
      {data.ticker}
    </svelte:fragment>
  </ListItem>

  <!-- CONTRACT ID -->
  <ListItem
    tooltipText={displayTooltips
      ? "The unique contract address of the token on the blockchain."
      : ""}
  >
    <svelte:fragment slot="term">Contract ID</svelte:fragment>
    <svelte:fragment slot="definition">
      {tokensContractID}
    </svelte:fragment>
  </ListItem>

  <!-- PRICE -->
  <ListItem
    tooltipText={displayTooltips
      ? "The current price of the token in USD."
      : ""}
  >
    <svelte:fragment slot="term">Price</svelte:fragment>
    <svelte:fragment slot="definition">
      {data.price}
    </svelte:fragment>
  </ListItem>
</DetailList>
