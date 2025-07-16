<svelte:options immutable={true} />

<script>
  import { onMount } from "svelte";
  import { Card, CopyButton, RelativeTime, Switch } from "$lib/dusk/components";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    createCurrencyFormatter,
    createFeeFormatter,
    luxToDusk,
  } from "$lib/dusk/currency";
  import {
    calculateAdaptiveCharCount,
    decodeHexString,
    makeClassName,
    middleEllipsis,
  } from "$lib/dusk/string";
  import {
    AppAnchor,
    DataCard,
    DataGuard,
    ListItem,
    StaleDataNotice,
    TransactionStatus,
    TransactionType,
  } from "$lib/components";
  import { addressCharPropertiesDefaults } from "$lib/constants";

  import "./TransactionDetails.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Transaction} */
  export let data;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {MarketData | null} */
  export let market;

  const formatter = createValueFormatter("en");
  const currencyFormatter = createCurrencyFormatter("en", "usd", 10);
  const feeFormatter = createFeeFormatter("en");

  const { minScreenWidth, maxScreenWidth, minCharCount, maxCharCount } =
    addressCharPropertiesDefaults;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {boolean} */
  let isPayloadToggled = false;

  /** @type {boolean} */
  let isMemoDecoded = false;

  /** @param {string} value */
  const withMiddleEllipsis = (value) =>
    middleEllipsis(
      value,
      calculateAdaptiveCharCount(
        screenWidth,
        minScreenWidth,
        maxScreenWidth,
        minCharCount,
        maxCharCount
      )
    );

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  $: classes = makeClassName(["transaction-details", className]);
</script>

<DataCard
  on:retry
  {data}
  {error}
  {loading}
  className={classes}
  title="Transaction Details"
>
  <dl class="transaction-details__list">
    <!-- TRANSACTION ID -->
    <ListItem tooltipText="The ID of the transaction">
      <svelte:fragment slot="term">ID</svelte:fragment>
      <svelte:fragment slot="definition">
        <AppAnchor
          className="transaction-details__list-anchor"
          href="/transactions/transaction?id={data.txid}"
          >{withMiddleEllipsis(data.txid)}</AppAnchor
        >
        <CopyButton rawValue={data.txid} name="Transaction ID" />
      </svelte:fragment>
    </ListItem>

    <!-- BLOCK HEIGHT -->
    <ListItem tooltipText="The block height this transaction belongs to">
      <svelte:fragment slot="term">block height</svelte:fragment>
      <svelte:fragment slot="definition">
        <AppAnchor
          className="transaction-details__list-anchor"
          href="/blocks/block?id={data.blockhash}"
          >{formatter(data.blockheight)}</AppAnchor
        ></svelte:fragment
      >
    </ListItem>

    <!-- TIMESTAMP -->
    <ListItem tooltipText="The date and time the transaction was created">
      <svelte:fragment slot="term">timestamp</svelte:fragment>
      <RelativeTime
        autoRefresh={true}
        className="transaction-details__list-timestamp"
        date={data.date}
        slot="definition"
        ><svelte:fragment let:relativeTime
          >{`${data.date.toUTCString()} (${relativeTime})`}</svelte:fragment
        ></RelativeTime
      >
    </ListItem>

    <!-- FROM -->
    {#if data.txtype.toLowerCase() === "moonlight" && data.from}
      <ListItem tooltipText="The sender of the transaction">
        <svelte:fragment slot="term">from</svelte:fragment>
        <svelte:fragment slot="definition">
          <AppAnchor
            className="transaction-details__list-anchor"
            href="/accounts/?key={data.from}"
            >{withMiddleEllipsis(data.from)}</AppAnchor
          >
          <CopyButton rawValue={data.from} name="Sender's address" />
        </svelte:fragment>
      </ListItem>
    {/if}

    <!-- TO -->
    {#if data.txtype.toLowerCase() === "moonlight" && data.to}
      <ListItem tooltipText="The receiver of the transaction">
        <svelte:fragment slot="term">to</svelte:fragment>
        <svelte:fragment slot="definition">
          <AppAnchor
            className="transaction-details__list-anchor"
            href="/accounts/?key={data.to}"
            >{withMiddleEllipsis(data.to)}</AppAnchor
          >
          <CopyButton rawValue={data.to} name="Receiver's address" />
        </svelte:fragment>
      </ListItem>
    {/if}

    <!-- AMOUNT -->
    {#if data.amount}
      <ListItem tooltipText="The transaction amount">
        <svelte:fragment slot="term">amount</svelte:fragment>
        <svelte:fragment slot="definition">
          <DataGuard data={market?.currentPrice.usd}>
            {`${feeFormatter(luxToDusk(data.amount))} DUSK (${currencyFormatter(luxToDusk(data.amount) * /** @type {number} */ (market?.currentPrice.usd))})`}
          </DataGuard>
          <StaleDataNotice /></svelte:fragment
        >
      </ListItem>
    {/if}

    <!-- TYPE -->
    <ListItem tooltipText="The transaction type">
      <svelte:fragment slot="term">type</svelte:fragment>
      <svelte:fragment slot="definition">
        <TransactionType {data} displayTooltips={true} />
      </svelte:fragment>
    </ListItem>

    <!-- STATUS -->
    <ListItem tooltipText="The transaction status">
      <svelte:fragment slot="term">status</svelte:fragment>
      <svelte:fragment slot="definition"
        ><TransactionStatus
          className="transaction-details__status explorer-badge"
          errorMessage={data.txerror}
          showErrorTooltip={true}
        /></svelte:fragment
      >
    </ListItem>

    <!-- TRANSACTION FEE -->
    <ListItem tooltipText="The fee paid for the transaction">
      <svelte:fragment slot="term">transaction fee</svelte:fragment>
      <svelte:fragment slot="definition">
        <DataGuard data={market?.currentPrice.usd}>
          {`${feeFormatter(luxToDusk(data.feepaid))} DUSK (${currencyFormatter(luxToDusk(data.feepaid) * /** @type {number} */ (market?.currentPrice.usd))})`}
        </DataGuard>
        <StaleDataNotice />
      </svelte:fragment>
    </ListItem>

    <!-- GAS PRICE -->
    <ListItem tooltipText="The transaction gas price">
      <svelte:fragment slot="term">gas price</svelte:fragment>
      <svelte:fragment slot="definition">
        <DataGuard data={market?.currentPrice.usd}>
          {`${feeFormatter(luxToDusk(data.gasprice))} DUSK (${currencyFormatter(luxToDusk(data.gasprice) * /** @type {number} */ (market?.currentPrice.usd))})`}
        </DataGuard>
        <StaleDataNotice />
      </svelte:fragment>
    </ListItem>

    <!-- GAS LIMIT -->
    <ListItem tooltipText="The transaction gas limit">
      <svelte:fragment slot="term">transaction gas limit</svelte:fragment>
      <svelte:fragment slot="definition">{data.gaslimit}</svelte:fragment>
    </ListItem>

    <!-- GAS SPENT -->
    <ListItem tooltipText="The amount of gas spent generating the transaction">
      <svelte:fragment slot="term">gas spent</svelte:fragment>
      <svelte:fragment slot="definition">{data.gasspent}</svelte:fragment>
    </ListItem>

    <!-- NONCE -->
    {#if data.txtype.toLowerCase() === "moonlight" && data.nonce}
      <ListItem tooltipText="The number of transactions sent from this address">
        <svelte:fragment slot="term">nonce</svelte:fragment>
        <svelte:fragment slot="definition">{data.nonce}</svelte:fragment>
      </ListItem>
    {/if}

    <!-- MEMO -->
    <ListItem tooltipText="Transaction reference and additional notes">
      <svelte:fragment slot="term">
        <div class="transaction-details__switch-wrapper">
          memo
          <Switch
            className="transaction-details__payload-switch"
            onSurface={true}
            bind:value={isMemoDecoded}
            disabled={!data.memo}
          />
        </div>
      </svelte:fragment>

      <svelte:fragment slot="definition">
        {#if isMemoDecoded}
          <Card onSurface={true} className="transaction-details__memo">
            <pre>{data.memo ? decodeHexString(data.memo) : "---"}</pre>
          </Card>
        {:else}
          <DataGuard data={data.memo}>{data.memo}</DataGuard>
        {/if}
      </svelte:fragment>
    </ListItem>

    <!-- PAYLOAD -->
    <ListItem tooltipText="The transaction payload">
      <svelte:fragment slot="term">
        <div class="transaction-details__switch-wrapper">
          payload
          <Switch
            className="transaction-details__payload-switch"
            onSurface={true}
            bind:value={isPayloadToggled}
          />
        </div>
      </svelte:fragment>

      <svelte:fragment slot="definition">
        {#if isPayloadToggled}
          <Card onSurface={true} className="transaction-details__payload">
            <pre>{data.payload
                ? JSON.stringify(data.payload, null, 2)
                : "---"}</pre>
          </Card>
        {/if}
      </svelte:fragment>
    </ListItem>

    {#if data.blobHashes.length}
      <!-- BLOB HASHES -->
      <ListItem tooltipText="The list of blob hashes">
        <svelte:fragment slot="term">Blob hashes</svelte:fragment>
        <svelte:fragment slot="definition">
          <ul class="transaction-details__blob-hashes">
            {#each data.blobHashes as blobHash (blobHash)}
              <li class="transaction-details__blob-hash">
                <pre>{withMiddleEllipsis(blobHash)}</pre>
                <CopyButton rawValue={blobHash} name="Blob hash" />
              </li>
            {/each}
          </ul>
        </svelte:fragment>
      </ListItem>
    {/if}
  </dl>
</DataCard>
