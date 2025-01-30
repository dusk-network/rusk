<script>
  import { page } from "$app/stores";
  import { tokens } from "$lib/mock-data";

  const url = new URL($page.url);
  const tokenName = url.searchParams.get("name");
  const tokenData = tokens.find((token) => token.token === tokenName);
</script>

<section class="token">
  {#if tokenData}
    <div class="token-overview-panel">
      <div>
        <h2>{tokenData.token}</h2>
        <p>Address: <b>{tokenData.contractId}</b></p>
      </div>
      <div class="token-overview-panel__details">
        <div>
          <p>{tokenData.totalCurrentSupply} {tokenData.ticker}</p>
          <small>Current Total Supply</small>
        </div>
        <div>
          <p>{tokenData.maxCirculatingSupply}</p>
          <small>Max Circulating Supply</small>
        </div>
        <div>
          <p>${tokenData.price}</p>
          <small>Price</small>
        </div>
      </div>
    </div>
  {:else}
    <p>Token not found</p>
  {/if}
</section>

<style>
  .token-overview-panel {
    display: flex;
    padding: 1rem 1.375rem;
    flex-direction: column;
    row-gap: 0.75rem;
    border-radius: 1.5rem;
    background-color: var(--surface-color);
    width: 100%;
    text-transform: uppercase;
  }

  .token-overview-panel__details {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 1rem;
    width: 100%;

    @media (min-width: 992px) {
      flex-direction: row;
    }
  }
</style>
